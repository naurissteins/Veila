use super::{AutomaticSuspend, Output, Request, State, providers::Providers};
use crate::services::suspend::{LockedSuspendState, SuspendDecision};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot, watch};
use veila_common::{ipc::CurtainControlMessage, session::SupervisorMessage};

pub(super) async fn run(
    mut changes: watch::Receiver<State>,
    requests: mpsc::Receiver<Request>,
    mut output: Output,
    path: Option<PathBuf>,
    commands: mpsc::Sender<Request>,
) {
    let Ok(wake) = output.wake.try_clone() else {
        return;
    };
    let Ok(fingerprint_wake) = output.wake.try_clone() else {
        return;
    };
    let fingerprint = super::fingerprint::Fingerprint::start(
        changes.clone(),
        Output {
            tx: output.tx.clone(),
            wake: fingerprint_wake,
        },
    )
    .await;
    // a missing timestamp means logind is unavailable or a sleep transition is active
    let (awake_since_tx, awake_since_rx) = watch::channel(None);
    tokio::spawn(super::logind::run(
        changes.clone(),
        requests,
        Output {
            tx: output.tx.clone(),
            wake,
        },
        fingerprint,
        awake_since_tx,
    ));
    let mut state = changes.borrow_and_update().clone();
    let mut providers = Providers::new(&state, path.as_deref());
    let mut suspend = LockedSuspendState::new(None, false, false);
    configure(&mut suspend, &state);
    suspend.arm(state.activity);
    let mut tick = tokio::time::interval(Duration::from_millis(500));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_status = None;
    let mut last_awake_since = None;
    let mut pending: Option<oneshot::Receiver<()>> = None;
    let mut media_updates = providers.media.subscribe();
    let mut weather_updates = providers.weather.subscribe();
    loop {
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() { break; }
                let next = changes.borrow_and_update().clone();
                if next.config != state.config {
                    if providers.update(&next) {
                        weather_updates = providers.weather.subscribe();
                    }
                    configure(&mut suspend, &next);
                }
                if next.generation != state.generation || next.activity != state.activity { suspend.note_activity(next.activity); }
                let refresh = next.revision != state.revision || next.generation != state.generation || next.confirmed != state.confirmed;
                state = next;
                if refresh {
                    providers.refresh(&state, path.as_deref(), &mut output).await;
                }
            }
            changed = media_updates.changed() => {
                if changed.is_err() { break; }
                providers.refresh(&state, path.as_deref(), &mut output).await;
            }
            Ok(()) = weather_updates.changed() => {
                providers.refresh(&state, path.as_deref(), &mut output).await;
            }
            _ = tick.tick() => {
                providers.refresh(&state, path.as_deref(), &mut output).await;
                let now = Instant::now();
                let awake_since = *awake_since_rx.borrow();
                if awake_since != last_awake_since && let Some(at) = awake_since {
                    suspend.note_activity(at);
                }
                last_awake_since = awake_since;
                // reconnect can discard the request by dropping its completion sender
                if let Some(done) = &mut pending
                    && !matches!(done.try_recv(), Err(oneshot::error::TryRecvError::Empty))
                {
                    pending = None;
                    suspend.note_activity(now);
                }
                let active = state.confirmed && awake_since.is_some() && pending.is_none();
                let decision = suspend.evaluate(
                    now,
                    active,
                    state.auth,
                    providers.battery.on_battery(),
                    providers.media.currently_playing(),
                );
                match (decision, awake_since) {
                    (SuspendDecision::Ready, Some(awake_since)) => {
                        let attempt = AutomaticSuspend {
                            generation: state.generation,
                            revision: state.revision,
                            activity: state.activity,
                            awake_since,
                        };
                        let (done, response) = oneshot::channel();
                        if commands.try_send(Request::Suspend { attempt, done }).is_ok() {
                            pending = Some(response);
                        }
                    }
                    (SuspendDecision::Skipped(reason), _) => {
                        if let Some(reason) = suspend.note_skip_reason(reason) {
                            tracing::info!(reason = reason.as_str(), "skipping locked idle suspend");
                        }
                    }
                    _ => suspend.clear_reported_skip_reason(),
                }
                let status = suspend.power_status_snapshot(now, active);
                if status != last_status {
                    output.message(state.generation, SupervisorMessage::Control(CurtainControlMessage::UpdatePowerStatus { snapshot: status.clone() })).await;
                    last_status = status;
                }
            }
        }
    }
}
fn configure(suspend: &mut LockedSuspendState, state: &State) {
    suspend.set_policy(
        state
            .config
            .lock
            .suspend_seconds
            .filter(|s| *s > 0)
            .map(Duration::from_secs),
        state.config.lock.suspend_only_on_battery,
        state.config.lock.skip_suspend_while_media_playing,
        Instant::now(),
        state.confirmed,
    );
}
