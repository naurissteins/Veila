use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::{io::AsyncWriteExt, net::UnixListener};
use veila_common::ipc::{CurtainStartupMessage, encode_message};

use super::{read_startup_message, startup_timeout};

fn unique_socket_path(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "veila-test-{label}-{}-{stamp}.sock",
        std::process::id()
    ))
}

#[test]
fn startup_timeout_uses_configured_acquisition_window_plus_margin() {
    assert_eq!(startup_timeout(9), std::time::Duration::from_secs(10));
}

#[tokio::test]
async fn reads_session_locked_startup_message() {
    let path = unique_socket_path("startup-session-locked");
    let listener = UnixListener::bind(&path).expect("bind startup socket");
    let mut client = tokio::net::UnixStream::connect(&path)
        .await
        .expect("connect startup socket");
    let (server, _) = listener.accept().await.expect("accept startup connection");
    let mut payload =
        encode_message(&CurtainStartupMessage::SessionLocked).expect("encode message");
    payload.push('\n');
    client
        .write_all(payload.as_bytes())
        .await
        .expect("write startup message");

    let message = read_startup_message(server)
        .await
        .expect("read startup message");

    assert_eq!(message, Some(CurtainStartupMessage::SessionLocked));
    drop(listener);
    std::fs::remove_file(&path).ok();
}
