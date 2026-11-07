use super::{Output, Request, State, providers::Providers};
use crate::services::suspend::{LockedSuspendState, SuspendDecision};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};
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
    tokio::spawn(super::logind::run(
        changes.clone(),
        requests,
        Output {
            tx: output.tx.clone(),
            wake,
        },
    ));
    let mut state = changes.borrow_and_update().clone();
    let mut providers = Providers::new(&state, path.as_deref());
    let mut suspend = LockedSuspendState::new(None, false, false);
    configure(&mut suspend, &state);
    suspend.arm(state.activity);
    let mut tick = tokio::time::interval(Duration::from_millis(500));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_status = None;
    let mut media_updates = providers.media.subscribe();
    loop {
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() { break; }
                let next = changes.borrow_and_update().clone();
                if next.config != state.config { providers.update(&next); configure(&mut suspend, &next); }
                if next.generation != state.generation || next.activity != state.activity { suspend.note_activity(next.activity); }
                state = next;
            }
            changed = media_updates.changed() => {
                if changed.is_err() { break; }
                providers.refresh(&state, path.as_deref(), &mut output).await;
            }
            _ = tick.tick() => {
                providers.refresh(&state, path.as_deref(), &mut output).await;
                let now = Instant::now();
                match suspend.evaluate(now, state.confirmed, state.auth, providers.battery.current_snapshot().as_ref(), providers.media.currently_playing()) {
                    SuspendDecision::Ready => {
                        if commands.try_send(Request::Suspend { generation: state.generation }).is_ok() { suspend.mark_requested(); }
                    }
                    SuspendDecision::Skipped(reason) => {
                        if let Some(reason) = suspend.note_skip_reason(reason) { tracing::info!(reason=reason.as_str(), "skipping locked idle suspend"); }
                    }
                    SuspendDecision::Pending => suspend.clear_reported_skip_reason(),
                }
                let status = suspend.power_status_snapshot(now, state.confirmed);
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
