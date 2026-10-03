use std::path::Path;
use veila_common::ipc::{DaemonControlResponse, LatencyReportMode};
use zbus::zvariant::OwnedObjectPath;

use super::{
    PendingStartup,
    requests::{Waiter, respond},
};
use crate::app::state::AppRuntime;

pub(in crate::app) struct DeferredRelock {
    trigger: &'static str,
    emergency: bool,
    latency: LatencyReportMode,
    pub(super) waiters: Vec<Waiter>,
}

impl PendingStartup {
    pub(super) fn defer_relock(
        &mut self,
        trigger: &'static str,
        emergency: bool,
        latency: LatencyReportMode,
    ) -> bool {
        if self.relock.is_some() {
            return false;
        }
        self.relock = Some(DeferredRelock {
            trigger,
            emergency,
            latency,
            waiters: Vec::new(),
        });
        true
    }

    pub(in crate::app) fn request_unlock(&mut self) {
        self.unlock_requested = true;
        if let Some(relock) = self.relock.take() {
            for waiter in relock.waiters {
                respond(
                    waiter.stream,
                    DaemonControlResponse::Error {
                        reason: "queued lock superseded by an authorized unlock".into(),
                    },
                );
            }
        }
    }

    pub(in crate::app) fn resume_relock(
        &mut self,
        runtime: &mut AppRuntime,
        connection: &zbus::Connection,
        session_path: &OwnedObjectPath,
        config_path: Option<&Path>,
    ) {
        if self.is_acquiring() || self.unlock_requested {
            return;
        }
        let Some(mut relock) = self.relock.take() else {
            return;
        };
        if runtime.state.is_active() {
            // Failed unlock delivery retained the original generation, which satisfies the new lock.
            for waiter in &mut relock.waiters {
                waiter.already_active = true;
            }
            self.waiters.extend(relock.waiters);
            if !self.is_pending() {
                self.finish_waiters(if runtime.active.is_some() {
                    Ok(None)
                } else {
                    Err("curtain ownership is unresolved; cannot confirm lock readiness".into())
                });
            }
        } else {
            self.begin(
                relock.trigger,
                runtime,
                connection,
                session_path,
                config_path,
                relock.emergency,
                relock.latency,
            );
            self.waiters.extend(relock.waiters);
        }
    }

    pub(super) fn waiter_count(&self) -> usize {
        self.waiters.len()
            + self.reloads.len()
            + self
                .relock
                .as_ref()
                .map_or(0, |relock| relock.waiters.len())
    }
}
