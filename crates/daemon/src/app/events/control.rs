use anyhow::Result;
use tokio::net::UnixStream;
use veila_common::ipc::{DaemonControlMessage, DaemonControlResponse};

use super::super::{
    helpers::{build_daemon_health, build_daemon_status, reload_config_response},
    runtime::control_socket_path,
    state::AppRuntime,
};
use crate::{DaemonOptions, adapters::ipc, domain::lock_state::LockState};

pub(crate) async fn handle_control_message(
    mut stream: UnixStream,
    message: DaemonControlMessage,
    options: &DaemonOptions,
    session_path: &str,
    runtime: &mut AppRuntime,
) -> Result<bool> {
    let (response, stop_requested) = match message {
        DaemonControlMessage::LockNow { .. } => (
            DaemonControlResponse::Error {
                reason: "lock request bypassed startup coordinator".into(),
            },
            false,
        ),
        DaemonControlMessage::Stop => stop_response(runtime.state, runtime.active.is_some()),
        DaemonControlMessage::Status => (
            DaemonControlResponse::Status(build_daemon_status(
                &runtime.state,
                session_path,
                runtime.active.is_some(),
                control_socket_path(&runtime.active),
                &runtime.loaded_config,
                runtime.last_reload_result.as_deref(),
                runtime.last_reload_unix_ms,
            )),
            false,
        ),
        DaemonControlMessage::Health => {
            (DaemonControlResponse::Health(build_daemon_health()), false)
        }
        DaemonControlMessage::ReloadConfig => (
            reload_config_response(
                options,
                &runtime.state,
                control_socket_path(&runtime.active),
                &mut runtime.loaded_config,
                &mut runtime.last_reload_result,
                &mut runtime.last_reload_unix_ms,
                &mut runtime.auth_policy,
                &mut runtime.auth_state,
                &mut runtime.suspend_state,
                &runtime.weather,
                &runtime.battery,
                &runtime.now_playing,
            )
            .await,
            false,
        ),
    };
    if let Err(error) = ipc::write_daemon_control_response(&mut stream, &response).await {
        tracing::warn!("failed to acknowledge daemon control request: {error:#}");
    }
    Ok(stop_requested)
}

fn stop_response(state: LockState, curtain_running: bool) -> (DaemonControlResponse, bool) {
    if state.is_active() || curtain_running {
        // The daemon must remain available to authenticate the current lock.
        return (
            DaemonControlResponse::Error {
                reason: "cannot stop Veila while the session is locked; unlock first".to_string(),
            },
            false,
        );
    }
    (DaemonControlResponse::Accepted, true)
}

#[cfg(test)]
mod tests {
    use veila_common::ipc::DaemonControlResponse;

    use super::{LockState, stop_response};

    #[test]
    fn stop_is_refused_during_every_active_lock_phase() {
        for state in [LockState::Locking, LockState::Locked, LockState::Unlocking] {
            let (response, stop_requested) = stop_response(state, true);
            assert!(matches!(response, DaemonControlResponse::Error { .. }));
            assert!(!stop_requested);
            assert!(matches!(
                stop_response(state, false),
                (DaemonControlResponse::Error { .. }, false)
            ));
        }
        assert_eq!(
            stop_response(LockState::Unlocked, false),
            (DaemonControlResponse::Accepted, true)
        );
        assert!(matches!(
            stop_response(LockState::Unlocked, true),
            (DaemonControlResponse::Error { .. }, false)
        ));
    }
}
