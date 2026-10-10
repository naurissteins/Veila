use super::{Milestone, PendingStartup, requests::Waiter};
use crate::{
    app::{
        runtime::{Fixture, LockActivation, monitor},
        state::AppRuntime,
    },
    domain::lock_state::LockState,
};
use std::{future::pending, time::Instant};
use tokio::{net::UnixStream, time::Duration};
use veila_auth::policy::AuthAdmission;
use veila_common::{
    AppConfig, LoadedConfig,
    ipc::{DaemonControlResponse, LockLatencyReport},
};

fn runtime() -> AppRuntime {
    AppRuntime::new(
        LoadedConfig {
            path: None,
            config: AppConfig::default(),
        },
        0,
        0,
    )
}

async fn secured(startup: &mut PendingStartup, runtime: &mut AppRuntime, fixture: &mut Fixture) {
    let activation = LockActivation {
        active: fixture.active.take().expect("active"),
        readiness: monitor(fixture.root.join("notify.sock"), Duration::from_secs(1)),
    };
    startup.acquiring = true;
    startup
        .complete(Milestone::Secured(Box::new(activation)), runtime)
        .await;
}

#[tokio::test]
async fn authentication_is_available_while_rich_readiness_is_pending() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime();
    let mut startup = PendingStartup::default();
    secured(&mut startup, &mut runtime, &mut fixture).await;
    assert_eq!(runtime.state, LockState::Locked);
    assert!(runtime.active.is_some());
    assert!(startup.is_pending());
    assert!(!startup.is_acquiring());
    assert!(matches!(
        runtime.auth_state.admit(Instant::now()),
        AuthAdmission::Allowed
    ));
}

#[tokio::test]
async fn readiness_does_not_reset_auth_backoff_or_in_flight_attempt() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime();
    let mut startup = PendingStartup::default();
    secured(&mut startup, &mut runtime, &mut fixture).await;
    runtime.auth_state.start_attempt();
    runtime.auth_state.finish_failure(Instant::now());
    startup.complete(Milestone::Ready(None), &mut runtime).await;
    assert!(matches!(
        runtime.auth_state.admit(Instant::now()),
        AuthAdmission::RateLimited(_)
    ));
    runtime.auth_state.start_attempt();
    startup.complete(Milestone::Ready(None), &mut runtime).await;
    assert!(runtime.auth_state.in_flight());
}

#[tokio::test]
async fn lost_generation_cancels_readiness_without_authorizing_unlock() {
    let mut fixture = Fixture::new();
    let mut runtime = runtime();
    let mut startup = PendingStartup::default();
    secured(&mut startup, &mut runtime, &mut fixture).await;
    crate::app::runtime::reset_runtime(
        &mut runtime.active,
        runtime.auth_policy,
        &mut runtime.auth_state,
    );
    startup.reconcile(&runtime);
    assert!(!startup.is_pending());
    assert_eq!(runtime.state, LockState::Locked);
    assert!(!fixture.root.join("notify.sock").exists());
    assert!(fixture.root.join("owner.json").exists());
}

#[tokio::test]
async fn replacing_generation_retires_old_readiness_only() {
    let mut fixture = Fixture::new();
    let mut other = Fixture::new();
    let mut runtime = runtime();
    let mut startup = PendingStartup::default();
    secured(&mut startup, &mut runtime, &mut fixture).await;
    runtime.active = other.active.take();
    startup.reconcile(&runtime);
    assert!(!startup.is_pending());
    assert!(runtime.active.is_some());
    assert!(other.root.join("auth.sock").exists());
}

#[tokio::test]
async fn acquisition_is_not_dropped_when_runtime_has_no_installed_generation() {
    let mut startup = PendingStartup {
        acquiring: true,
        operation: Some(Box::pin(pending())),
        ..Default::default()
    };
    let mut runtime = runtime();
    runtime.state = LockState::Locking;
    startup.unlock_requested = true;
    startup.reconcile(&runtime);
    assert!(startup.is_acquiring());
    assert!(startup.is_pending());
    assert!(startup.unlock_requested);
    assert!(!crate::app::events::ShutdownGate::default().request(runtime.state, false));
}

#[tokio::test]
async fn acquisition_failure_preserves_explicit_unresolved_authority() {
    let mut startup = PendingStartup::default();
    let mut runtime = runtime();
    startup
        .complete(
            Milestone::Failed {
                state: LockState::Locking,
                error: anyhow::anyhow!("unresolved owner"),
            },
            &mut runtime,
        )
        .await;
    assert_eq!(runtime.state, LockState::Locking);
    assert!(!startup.is_pending());
    assert!(runtime.active.is_none());
}

#[tokio::test]
async fn confirmed_readiness_acknowledges_first_and_duplicate_waiters_differently() {
    let mut startup = PendingStartup::default();
    let mut clients = Vec::new();
    for already_active in [false, true] {
        let (client, server) = UnixStream::pair().expect("pair");
        clients.push(client);
        startup.waiters.push(Waiter {
            stream: server,
            already_active,
        });
    }
    startup.finish_waiters(Ok(Some(LockLatencyReport::default())));
    for (client, duplicate) in clients.iter_mut().zip([false, true]) {
        let line = crate::adapters::ipc::read_ipc_line(client, "response")
            .await
            .expect("read")
            .expect("line");
        let response: DaemonControlResponse =
            veila_common::ipc::decode_message(&line).expect("decode");
        assert!(
            matches!(response, DaemonControlResponse::Locked { already_active, latency_report } if already_active == duplicate && latency_report.is_some() != duplicate)
        );
    }
}

#[tokio::test]
async fn later_unlock_supersedes_a_queued_relock() {
    let mut startup = PendingStartup::default();
    startup.request_unlock();
    assert!(startup.defer_relock(
        "logind",
        false,
        veila_common::ipc::LatencyReportMode::Disabled
    ));
    assert!(!startup.defer_relock(
        "idle",
        false,
        veila_common::ipc::LatencyReportMode::Disabled
    ));
    let (mut client, stream) = UnixStream::pair().expect("pair");
    startup
        .relock
        .as_mut()
        .expect("queued relock")
        .waiters
        .push(Waiter {
            stream,
            already_active: true,
        });
    startup.request_unlock();
    assert!(startup.relock.is_none());
    assert!(startup.unlock_requested);
    let line = crate::adapters::ipc::read_ipc_line(&mut client, "response")
        .await
        .expect("read")
        .expect("response");
    let response: DaemonControlResponse = veila_common::ipc::decode_message(&line).expect("decode");
    assert!(matches!(response, DaemonControlResponse::Error { .. }));
}
