use super::{UNLOCK_DELIVERY_ATTEMPTS, UNLOCK_RETRY_DELAY, deliver_unlock};
use std::{
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixListener;

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

#[tokio::test]
async fn retries_before_reporting_an_undeliverable_unlock() {
    let path = unique_socket_path("unlock-missing");
    let started_at = Instant::now();

    deliver_unlock(&path, Some(1))
        .await
        .expect_err("a missing control socket must not report success");

    // Retry backoff proves delivery did not stop after the first failure.
    let minimum = UNLOCK_RETRY_DELAY * (UNLOCK_DELIVERY_ATTEMPTS - 1);
    assert!(
        started_at.elapsed() >= minimum,
        "expected at least {minimum:?} of retry backoff, took {:?}",
        started_at.elapsed()
    );
}

#[tokio::test]
async fn delivers_unlock_to_a_listening_curtain() {
    let path = unique_socket_path("unlock-delivered");
    let listener = UnixListener::bind(&path).expect("bind control socket");

    deliver_unlock(&path, Some(7))
        .await
        .expect("unlock should reach a listening curtain");

    drop(listener);
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn undeliverable_unlock_keeps_the_complete_resource_bundle_usable() {
    use crate::{
        adapters::ownership,
        app::runtime::{AuthResult, Fixture},
    };

    let mut fixture = Fixture::new();
    let active = fixture.active.as_mut().expect("active fixture");
    let owner_path = active.curtain.owner_path().to_path_buf();
    let sender = active.auth_sender.clone();
    assert!(matches!(
        super::stop_active_curtain(&mut active.curtain, &active.control_socket_path, Some(9)).await,
        super::CurtainStop::UnlockUndeliverable
    ));
    let crate::adapters::process::CurtainHandle::Adopted {
        pid, start_ticks, ..
    } = active.curtain
    else {
        panic!("fixture changed curtain kind");
    };
    assert!(ownership::process_matches(pid, start_ticks).expect("identity"));
    assert!(owner_path.exists());
    assert!(active.auth_socket_path.exists());
    assert!(active.control_socket_path.exists());
    assert!(!sender.is_closed());
    sender
        .send(AuthResult::Rejected {
            attempt_id: 9,
            started_at: Instant::now(),
            elapsed_ms: 0,
        })
        .expect("channel remains usable");
    assert!(matches!(
        active.auth_results.try_recv(),
        Ok(AuthResult::Rejected { attempt_id: 9, .. })
    ));
    let connection = tokio::net::UnixStream::connect(&active.auth_socket_path)
        .await
        .expect("auth socket remains reachable");
    let _ = active
        .auth_listener
        .accept()
        .await
        .expect("auth listener remains usable");
    drop(connection);
}
