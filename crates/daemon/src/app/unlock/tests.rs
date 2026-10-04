use super::*;
use crate::{app::runtime::Fixture, domain::auth::AuthAdmission};
use tokio::{io::AsyncReadExt, net::UnixListener};
use veila_common::{AppConfig, LoadedConfig};

fn runtime(fixture: &mut Fixture) -> AppRuntime {
    let mut runtime = AppRuntime::new(
        LoadedConfig {
            path: None,
            config: AppConfig::default(),
        },
        0,
        0,
    );
    runtime.active = fixture.active.take();
    runtime.state = LockState::Locked;
    runtime
}

#[tokio::test]
async fn repeated_unlock_keeps_one_pinned_delivery_and_original_attempt() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    let mut unlock = PendingUnlock::default();
    std::fs::remove_file(fixture.root.join("control.sock")).expect("remove placeholder");
    let listener = UnixListener::bind(fixture.root.join("control.sock")).expect("listen");
    let success = AuthResult::Succeeded {
        attempt_id: 42,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };
    assert!(unlock.begin(&mut runtime, Some(success)));
    assert!(unlock.begin(&mut runtime, None));
    assert_eq!(unlock.auth_result, Some(success));
    let progress = unlock.next().await;
    assert!(!unlock.advance(progress, &mut runtime).await);
    let (mut stream, _) = listener.accept().await.expect("accept");
    let mut data = Vec::new();
    stream.read_to_end(&mut data).await.expect("message");
    assert_eq!(data, b"{\"Unlock\":{\"attempt_id\":42}}\n");
    assert!(
        tokio::time::timeout(Duration::from_millis(10), listener.accept())
            .await
            .is_err()
    );
    assert_eq!(runtime.state, LockState::Unlocking);
    assert!(unlock.waiting_exit);
    assert!(runtime.active.is_some());
    assert!(fixture.root.join("owner.json").exists());
}

#[tokio::test]
async fn cancelled_selection_preserves_delivery_retries_and_resources() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    let mut unlock = PendingUnlock::default();
    unlock.begin(&mut runtime, None);
    for _ in 0..3 {
        assert!(
            tokio::time::timeout(Duration::from_millis(10), unlock.next())
                .await
                .is_err()
        );
        assert_eq!(runtime.state, LockState::Unlocking);
        assert!(runtime.active.is_some());
    }
    let progress = unlock.next().await;
    assert!(unlock.advance(progress, &mut runtime).await);
    assert_eq!(runtime.state, LockState::Locked);
    assert!(!unlock.is_pending());
    assert!(runtime.active.is_some());
    assert!(fixture.root.join("auth.sock").exists());
    assert!(fixture.root.join("owner.json").exists());
}

#[tokio::test]
async fn adopted_deadline_retains_owner_without_signaling_the_process() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    let mut unlock = PendingUnlock::default();
    unlock.begin(&mut runtime, None);
    unlock
        .advance(Progress::Delivered(Ok(())), &mut runtime)
        .await;
    assert!(unlock.advance(Progress::Deadline, &mut runtime).await);
    assert_eq!(runtime.state, LockState::Locked);
    assert!(runtime.active.is_some());
    assert!(fixture.root.join("owner.json").exists());
}

#[tokio::test]
async fn unexpected_generation_cannot_clear_replacement_or_its_auth_backoff() {
    let mut fixture = Fixture::new();
    let mut replacement = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    let mut unlock = PendingUnlock::default();
    unlock.begin(&mut runtime, None);
    runtime.active = replacement.active.take();
    runtime.state = LockState::Locked;
    runtime.auth_state.finish_failure(Instant::now());
    assert!(
        unlock
            .advance(Progress::Delivered(Ok(())), &mut runtime)
            .await
    );
    assert_eq!(runtime.state, LockState::Locked);
    assert!(replacement.root.join("owner.json").exists());
    assert!(matches!(
        runtime.auth_state.admit(Instant::now()),
        AuthAdmission::RateLimited(_)
    ));
}

#[tokio::test]
async fn send_success_alone_cannot_release_resources_or_clear_authority() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    let mut unlock = PendingUnlock::default();
    unlock.begin(&mut runtime, None);
    assert!(!unlock.confirm_release(&mut runtime));
    assert_eq!(runtime.state, LockState::Locked);
    assert!(runtime.active.is_some());
    assert!(fixture.root.join("owner.json").exists());
}

#[tokio::test]
async fn confirmed_exit_cleans_resources_before_hint_completion_and_relock() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    let sender = runtime.active.as_ref().expect("active").auth_sender.clone();
    let mut unlock = PendingUnlock::default();
    unlock.begin(&mut runtime, None);
    unlock
        .advance(Progress::Delivered(Ok(())), &mut runtime)
        .await;
    assert!(unlock.confirm_release(&mut runtime));
    assert_eq!(runtime.state, LockState::Unlocking);
    assert!(runtime.active.is_none());
    assert!(sender.is_closed());
    for name in ["owner.json", "auth.sock", "control.sock"] {
        assert!(!fixture.root.join(name).exists());
    }
    assert!(!crate::app::events::ShutdownGate::default().request(runtime.state, false));
    assert!(unlock.advance(Progress::HintCleared, &mut runtime).await);
    assert_eq!(runtime.state, LockState::Unlocked);
}

#[tokio::test]
async fn unresolved_owner_refuses_a_new_unlock_operation() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    crate::app::runtime::reset_runtime(
        &mut runtime.active,
        runtime.auth_policy,
        &mut runtime.auth_state,
    );
    let mut unlock = PendingUnlock::default();
    assert!(!unlock.begin(&mut runtime, None));
    assert_eq!(runtime.state, LockState::Locked);
    assert!(fixture.root.join("owner.json").exists());
}

#[tokio::test]
async fn spawned_timeout_signals_once_then_retains_resources_if_exit_is_unconfirmed() {
    use tokio::{io::AsyncBufReadExt, process::Command};
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    let mut child = Command::new("sh")
        .args(["-c", "trap '' TERM; echo ready; exec sleep 60"])
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("child");
    let mut ready = tokio::io::BufReader::new(child.stdout.take().expect("stdout"));
    let mut line = String::new();
    ready.read_line(&mut line).await.expect("ready");
    runtime.active.as_mut().expect("active").curtain = CurtainHandle::Spawned {
        child,
        owner_path: fixture.root.join("owner.json"),
    };
    let mut unlock = PendingUnlock::default();
    unlock.begin(&mut runtime, None);
    unlock
        .advance(Progress::Delivered(Ok(())), &mut runtime)
        .await;
    assert!(!unlock.advance(Progress::Deadline, &mut runtime).await);
    assert!(unlock.terminating);
    assert_eq!(runtime.state, LockState::Unlocking);
    assert!(unlock.advance(Progress::Deadline, &mut runtime).await);
    assert_eq!(runtime.state, LockState::Locked);
    assert!(fixture.root.join("owner.json").exists());
    if let CurtainHandle::Spawned { child, .. } =
        &mut runtime.active.as_mut().expect("active").curtain
    {
        child.kill().await.expect("cleanup child");
    }
}

#[tokio::test]
async fn failed_delivery_preserves_in_flight_auth_until_its_queued_result_is_processed() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime(&mut fixture);
    runtime.auth_state.start_attempt();
    let mut unlock = PendingUnlock::default();
    unlock.begin(&mut runtime, None);
    let result = AuthResult::Rejected {
        attempt_id: 12,
        started_at: Instant::now(),
        elapsed_ms: 0,
    };
    runtime
        .active
        .as_ref()
        .expect("active")
        .auth_sender
        .send(result)
        .expect("queued result");
    assert!(
        unlock
            .advance(
                Progress::Delivered(Err(anyhow::anyhow!("failed"))),
                &mut runtime
            )
            .await
    );
    assert!(runtime.auth_state.in_flight());
    let queued = runtime
        .active
        .as_mut()
        .expect("active")
        .auth_results
        .try_recv()
        .expect("retained result");
    assert!(crate::app::events::handle_auth_result(&mut runtime.auth_state, queued).is_none());
    assert!(!runtime.auth_state.in_flight());
    assert!(matches!(
        runtime.auth_state.admit(Instant::now()),
        AuthAdmission::RateLimited(_)
    ));
}
