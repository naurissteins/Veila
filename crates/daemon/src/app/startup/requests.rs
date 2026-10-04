use tokio::{
    net::UnixStream,
    time::{Duration, timeout},
};
use veila_common::ipc::{DaemonControlMessage, DaemonControlResponse};
use zbus::zvariant::OwnedObjectPath;

use super::{
    super::{connections::ControlConnection, state::AppRuntime},
    PendingStartup,
};
use crate::adapters::ipc;

const MAX_STARTUP_WAITERS: usize = 32;

pub(super) struct Waiter {
    pub(super) stream: UnixStream,
    pub(super) already_active: bool,
}

impl PendingStartup {
    pub(crate) async fn handle_request(
        &mut self,
        request: ControlConnection,
        runtime: &mut AppRuntime,
        connection: &zbus::Connection,
        session_path: &OwnedObjectPath,
        config_path: Option<&std::path::Path>,
    ) -> Option<ControlConnection> {
        match request.message {
            DaemonControlMessage::LockNow {
                wait_ready,
                force_emergency_ui,
                latency_report,
                sleep_transition,
            } => {
                if runtime.state.is_active()
                    && runtime.active.is_none()
                    && !self.is_pending()
                    && !self.unlock_requested
                {
                    respond(
                        request.stream,
                        DaemonControlResponse::Error {
                            reason:
                                "curtain ownership is unresolved; cannot confirm lock readiness"
                                    .into(),
                        },
                    );
                    return None;
                }
                if sleep_transition {
                    runtime.fingerprint.pause_for_sleep().await;
                }
                let after_unlock = self.unlock_requested;
                let started = if after_unlock {
                    self.defer_relock("forwarded", force_emergency_ui, latency_report)
                } else {
                    self.begin(
                        "forwarded",
                        runtime,
                        connection,
                        session_path,
                        config_path,
                        force_emergency_ui,
                        latency_report,
                    )
                };
                if !wait_ready {
                    respond(request.stream, DaemonControlResponse::Accepted);
                } else if self.is_pending() || after_unlock {
                    if self.waiter_count() >= MAX_STARTUP_WAITERS {
                        respond(
                            request.stream,
                            DaemonControlResponse::Error {
                                reason: "too many requests waiting for curtain startup".into(),
                            },
                        );
                    } else {
                        let waiter = Waiter {
                            stream: request.stream,
                            already_active: !started,
                        };
                        if after_unlock {
                            if let Some(relock) = &mut self.relock {
                                relock.waiters.push(waiter);
                            }
                        } else {
                            self.waiters.push(waiter);
                        }
                    }
                } else {
                    respond(
                        request.stream,
                        DaemonControlResponse::Locked {
                            already_active: true,
                            latency_report: None,
                        },
                    );
                }
                None
            }
            DaemonControlMessage::ReloadConfig if self.is_pending() || self.unlock_requested => {
                if self.waiter_count() >= MAX_STARTUP_WAITERS {
                    respond(
                        request.stream,
                        DaemonControlResponse::Error {
                            reason: "too many requests waiting for curtain startup".into(),
                        },
                    );
                } else {
                    self.reloads.push_back(request);
                }
                None
            }
            _ => Some(request),
        }
    }
}

pub(super) fn respond(mut stream: UnixStream, response: DaemonControlResponse) {
    // A disconnected readiness waiter cannot delay authentication or other waiters.
    tokio::spawn(async move {
        match timeout(
            Duration::from_secs(2),
            ipc::write_daemon_control_response(&mut stream, &response),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::debug!("failed to acknowledge startup request: {error:#}"),
            Err(_) => tracing::debug!("startup acknowledgement timed out"),
        }
    });
}
