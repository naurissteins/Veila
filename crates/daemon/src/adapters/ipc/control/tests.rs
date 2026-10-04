use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    task::JoinHandle,
    time::{advance, pause, resume},
};
use veila_common::ipc::{
    DaemonControlMessage, DaemonControlResponse, IPC_MAX_LINE_BYTES, LatencyReportMode,
};

use super::{
    DEFERRED_RESPONSE_TIMEOUT, RESPONSE_TIMEOUT, SEND_TIMEOUT, response_timeout,
    send_daemon_control_message,
};

fn lock_message(wait_ready: bool) -> DaemonControlMessage {
    DaemonControlMessage::LockNow {
        wait_ready,
        force_emergency_ui: false,
        latency_report: LatencyReportMode::Disabled,
        sleep_transition: false,
    }
}

struct Peer {
    root: PathBuf,
    stream: UnixStream,
}

impl Drop for Peer {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn fixture_root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "veila-control-client-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    fs::create_dir(&root).expect("private fixture directory");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("permissions");
    root
}

async fn request(
    message: DaemonControlMessage,
) -> (JoinHandle<anyhow::Result<DaemonControlResponse>>, Peer) {
    let root = fixture_root();
    let path = root.join("control.sock");
    let listener = UnixListener::bind(&path).expect("fixture listener");
    let sent = message.clone();
    let client = tokio::spawn(async move { send_daemon_control_message(&path, &sent).await });
    let (mut stream, _) = tokio::time::timeout(SEND_TIMEOUT, listener.accept())
        .await
        .expect("client connected in time")
        .expect("accepted client");
    let received = tokio::time::timeout(
        SEND_TIMEOUT,
        crate::adapters::ipc::read_daemon_control_message(&mut stream),
    )
    .await
    .expect("request arrived in time")
    .expect("valid request");
    assert_eq!(received, Some(message));
    (client, Peer { root, stream })
}

#[test]
fn ordinary_commands_have_a_short_response_budget() {
    for message in [
        DaemonControlMessage::Status,
        DaemonControlMessage::Health,
        DaemonControlMessage::Stop,
        lock_message(false),
    ] {
        assert_eq!(response_timeout(&message), RESPONSE_TIMEOUT);
    }
}

#[test]
fn readiness_and_reload_have_a_separate_deferred_budget() {
    for message in [lock_message(true), DaemonControlMessage::ReloadConfig] {
        assert_eq!(response_timeout(&message), DEFERRED_RESPONSE_TIMEOUT);
    }
}

#[tokio::test]
async fn silent_peer_times_out_and_sees_only_request_then_eof() {
    let (client, mut peer) = request(lock_message(false)).await;
    pause();
    advance(RESPONSE_TIMEOUT + std::time::Duration::from_millis(1)).await;
    let error = client
        .await
        .expect("client task")
        .expect_err("missing response");
    assert!(error.to_string().contains("after 5 seconds"), "{error:#}");
    assert!(error.to_string().contains("request may still complete"));
    let mut byte = [0];
    assert_eq!(peer.stream.read(&mut byte).await.expect("closed client"), 0);
}

#[tokio::test]
async fn deferred_commands_can_reply_after_the_ordinary_budget() {
    for message in [lock_message(true), DaemonControlMessage::ReloadConfig] {
        let (client, mut peer) = request(message).await;
        pause();
        advance(RESPONSE_TIMEOUT + std::time::Duration::from_secs(1)).await;
        assert!(!client.is_finished());
        peer.stream
            .write_all(b"\"Accepted\"\n")
            .await
            .expect("delayed response");
        assert_eq!(
            client.await.expect("client task").expect("response"),
            DaemonControlResponse::Accepted
        );
        resume();
    }
}

#[tokio::test]
async fn deferred_commands_still_have_a_total_response_deadline() {
    for message in [lock_message(true), DaemonControlMessage::ReloadConfig] {
        let (client, _peer) = request(message).await;
        pause();
        advance(DEFERRED_RESPONSE_TIMEOUT + std::time::Duration::from_millis(1)).await;
        let error = client
            .await
            .expect("client task")
            .expect_err("missing response");
        assert!(error.to_string().contains("after 120 seconds"), "{error:#}");
        resume();
    }
}

#[tokio::test]
async fn partial_response_bytes_do_not_extend_the_deadline() {
    let (client, mut peer) = request(DaemonControlMessage::Status).await;
    pause();
    advance(std::time::Duration::from_secs(2)).await;
    peer.stream.write_all(b"\"").await.expect("first fragment");
    advance(std::time::Duration::from_secs(2)).await;
    peer.stream
        .write_all(b"Accepted\"")
        .await
        .expect("second fragment");
    advance(std::time::Duration::from_millis(1001)).await;
    let error = client
        .await
        .expect("client task")
        .expect_err("unterminated response");
    assert!(error.to_string().contains("after 5 seconds"), "{error:#}");
}

#[tokio::test]
async fn complete_fragmented_response_succeeds_before_deadline() {
    let (client, mut peer) = request(DaemonControlMessage::Stop).await;
    pause();
    advance(std::time::Duration::from_secs(2)).await;
    peer.stream
        .write_all(b"\"Acc")
        .await
        .expect("first fragment");
    advance(std::time::Duration::from_secs(1)).await;
    peer.stream
        .write_all(b"epted\"\n")
        .await
        .expect("second fragment");
    assert_eq!(
        client.await.expect("client task").expect("response"),
        DaemonControlResponse::Accepted
    );
}

#[tokio::test]
async fn preserves_daemon_error_responses() {
    let (client, mut peer) = request(DaemonControlMessage::Stop).await;
    peer.stream
        .write_all(b"{\"Error\":{\"reason\":\"session is locked\"}}\n")
        .await
        .expect("response");
    assert_eq!(
        client.await.expect("client task").expect("response"),
        DaemonControlResponse::Error {
            reason: "session is locked".into()
        }
    );
}

#[tokio::test]
async fn rejects_eof_without_a_response() {
    let (client, mut peer) = request(DaemonControlMessage::Status).await;
    peer.stream.shutdown().await.expect("server close");
    let error = client.await.expect("client task").expect_err("empty EOF");
    assert_eq!(
        error.to_string(),
        "daemon closed control socket without a response"
    );
}

#[tokio::test]
async fn rejects_eof_before_response_newline() {
    let (client, mut peer) = request(DaemonControlMessage::Status).await;
    peer.stream
        .write_all(b"\"Accepted\"")
        .await
        .expect("response without delimiter");
    peer.stream.shutdown().await.expect("server close");
    let error = client
        .await
        .expect("client task")
        .expect_err("truncated response");
    assert!(
        error.to_string().contains("ended before newline"),
        "{error:#}"
    );
}

#[tokio::test]
async fn rejects_invalid_response_messages() {
    let (client, mut peer) = request(DaemonControlMessage::Status).await;
    peer.stream
        .write_all(b"\"Unexpected\"\n")
        .await
        .expect("invalid response");
    let error = client
        .await
        .expect("client task")
        .expect_err("invalid response");
    assert_eq!(error.to_string(), "invalid daemon control response");
}

#[tokio::test]
async fn rejects_non_utf8_response_frames() {
    let (client, mut peer) = request(DaemonControlMessage::Status).await;
    peer.stream
        .write_all(b"\xff\n")
        .await
        .expect("non-UTF8 response");
    let error = client
        .await
        .expect("client task")
        .expect_err("invalid response");
    assert!(error.to_string().contains("is not UTF-8"), "{error:#}");
}

#[tokio::test]
async fn rejects_oversized_response_frames() {
    let (client, mut peer) = request(DaemonControlMessage::Status).await;
    let _ = peer
        .stream
        .write_all(&vec![b'x'; IPC_MAX_LINE_BYTES + 1])
        .await;
    let error = client
        .await
        .expect("client task")
        .expect_err("oversized response");
    assert!(
        error.to_string().contains("exceeds 65536 bytes"),
        "{error:#}"
    );
}

#[tokio::test]
async fn preserves_immediate_connection_errors() {
    let missing =
        std::env::temp_dir().join(format!("missing-veila-control-{}.sock", std::process::id()));
    let error = send_daemon_control_message(&missing, &DaemonControlMessage::Status)
        .await
        .expect_err("missing socket");
    assert!(
        error
            .to_string()
            .starts_with("failed to connect to daemon socket"),
        "{error:#}"
    );
}

#[tokio::test]
async fn full_listener_backlog_cannot_leave_connect_waiting_forever() {
    let root = fixture_root();
    let path = root.join("backlog.sock");
    let listener = std::os::unix::net::UnixListener::bind(&path).expect("listener");
    nix::sys::socket::listen(
        &listener,
        nix::sys::socket::Backlog::new(1).expect("backlog"),
    )
    .expect("small backlog");
    let first = UnixStream::connect(&path)
        .await
        .expect("first queued client");
    let _first = Peer {
        root,
        stream: first,
    };
    let _second = UnixStream::connect(&path)
        .await
        .expect("second queued client");
    pause();
    let started = tokio::time::Instant::now();
    let error = send_daemon_control_message(&path, &DaemonControlMessage::Status)
        .await
        .expect_err("saturated backlog");
    assert!(tokio::time::Instant::now() - started <= SEND_TIMEOUT);
    let reason = error.to_string();
    assert!(
        reason.starts_with("failed to connect to daemon socket")
            || reason.starts_with("timed out sending daemon control request"),
        "{error:#}",
    );
}
