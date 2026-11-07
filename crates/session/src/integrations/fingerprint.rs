use super::{Event, Output, State};
use crate::services::fingerprint::{FingerprintHandle, FingerprintMatch};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc, watch};
use veila_common::{ipc::CurtainControlMessage, session::SupervisorMessage};

#[derive(Clone)]
pub(super) struct Fingerprint(Arc<Mutex<FingerprintHandle>>);

impl Fingerprint {
    pub(super) async fn start(changes: watch::Receiver<State>, output: Output) -> Self {
        let handle = Self(Arc::new(Mutex::new(FingerprintHandle::new())));
        handle.pause().await;
        tokio::spawn(run(handle.clone(), changes, output));
        handle
    }
    pub(super) async fn pause(&self) {
        self.0.lock().await.pause_for_sleep().await;
    }
    pub(super) async fn resume(&self) {
        self.0.lock().await.resume_after_sleep();
    }
    pub(super) async fn stop(&self) {
        let mut handle = self.0.lock().await;
        handle.pause_for_sleep().await;
        handle.stop().await;
    }
}

async fn run(handle: Fingerprint, mut changes: watch::Receiver<State>, mut output: Output) {
    let username = match nix::unistd::User::from_uid(nix::unistd::Uid::current()) {
        Ok(Some(user)) => user.name,
        result => {
            tracing::warn!(
                ?result,
                "fingerprint username unavailable; local PAM remains active"
            );
            return;
        }
    };
    let mut state = changes.borrow_and_update().clone();
    let (matches, mut results) = mpsc::unbounded_channel::<FingerprintMatch>();
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            changed = changes.changed() => {
                if changed.is_err() { break; }
                let next = changes.borrow_and_update().clone();
                if next.generation != state.generation || next.revision != state.revision || !next.confirmed {
                    // Replacements and reloads revoke queued verdicts without resetting the lock's limit.
                    handle.0.lock().await.cancel_attempt().await;
                    while results.try_recv().is_ok() {}
                }
                state = next;
                forward_status(&handle, &state, &mut output).await;
            }
            matched = results.recv() => {
                let Some(matched) = matched else { break; };
                let authorization = handle.0.lock().await.authorization(matched);
                if let Some(authorization) = authorization {
                    output.send(Event::FingerprintMatch {
                        generation: state.generation, revision: state.revision,
                        attempt_id: matched.attempt_id, authorization,
                    }).await;
                }
            }
            _ = tick.tick(), if state.confirmed && state.config.fingerprint.enabled => {
                handle.0.lock().await.update(true, true,
                    state.config.fingerprint.max_failed_attempts, &username, Some(matches.clone())).await;
                forward_status(&handle, &state, &mut output).await;
            }
        }
    }
    handle.stop().await;
}

async fn forward_status(handle: &Fingerprint, state: &State, output: &mut Output) {
    let mut fingerprint = handle.0.lock().await;
    let mut statuses = Vec::new();
    while let Some(status) = fingerprint.take_status() {
        statuses.push(status);
    }
    drop(fingerprint);
    for status in statuses {
        tracing::debug!(?status, "supervisor fingerprint status");
        output
            .send(Event::Message {
                generation: state.generation,
                revision: Some(state.revision),
                message: SupervisorMessage::Control(
                    CurtainControlMessage::UpdateFingerprintStatus { status },
                ),
            })
            .await;
    }
}
