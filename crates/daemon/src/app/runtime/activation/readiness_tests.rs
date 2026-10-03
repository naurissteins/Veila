use super::readiness::RichReadiness;
use std::time::Instant;
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    time::{Duration, Instant as TokioInstant, timeout},
};
use veila_common::ipc::{CurtainStartupMessage, LockLatencyReport, encode_message};

pub(crate) fn monitor(path: std::path::PathBuf, duration: Duration) -> RichReadiness {
    RichReadiness {
        listener: UnixListener::bind(&path).expect("notification listener"),
        path,
        deadline: TokioInstant::now() + duration,
        trigger: "test",
        activation_started_at: Instant::now(),
        ready_wait_started_at: Instant::now(),
        report: Some(LockLatencyReport::default()),
    }
}

#[tokio::test]
async fn deadline_covers_a_silent_notification_connection() {
    let fixture = crate::app::runtime::Fixture::new();
    let path = fixture.root.join("notify.sock");
    let readiness = monitor(path.clone(), Duration::from_millis(30));
    let _silent = UnixStream::connect(&path).await.expect("connect");
    let report = timeout(Duration::from_millis(200), readiness.wait())
        .await
        .expect("deadline")
        .expect("report");
    assert!(report.curtain.is_none());
    assert!(!path.exists());
    assert!(fixture.root.join("auth.sock").exists());
    assert!(fixture.root.join("owner.json").exists());
}

#[tokio::test]
async fn partial_read_survives_other_event_loop_selections() {
    let fixture = crate::app::runtime::Fixture::new();
    let path = fixture.root.join("notify.sock");
    let readiness = monitor(path.clone(), Duration::from_secs(1));
    let mut client = UnixStream::connect(&path).await.expect("connect");
    let mut payload = encode_message(&CurtainStartupMessage::Ready {
        latency_report: None,
    })
    .expect("encode");
    payload.push('\n');
    let middle = payload.len() / 2;
    client
        .write_all(&payload.as_bytes()[..middle])
        .await
        .expect("partial write");
    let mut operation = Box::pin(readiness.wait());
    for _ in 0..3 {
        tokio::select! {
            _ = &mut operation => panic!("incomplete frame completed readiness"),
            _ = tokio::time::sleep(Duration::from_millis(5)) => {},
        }
    }
    client
        .write_all(&payload.as_bytes()[middle..])
        .await
        .expect("remaining write");
    let report = operation.await.expect("report");
    assert!(report.activation_total_ms < 200);
    assert!(!path.exists());
}

#[tokio::test]
async fn dropping_readiness_removes_only_its_notification_socket() {
    let fixture = crate::app::runtime::Fixture::new();
    let path = fixture.root.join("notify.sock");
    drop(monitor(path.clone(), Duration::from_secs(1)));
    assert!(!path.exists());
    assert!(fixture.root.join("auth.sock").exists());
    assert!(fixture.root.join("control.sock").exists());
    assert!(fixture.root.join("owner.json").exists());
}
