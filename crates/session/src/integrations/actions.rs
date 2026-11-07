use super::{Event, Output, State};
use crate::adapters::logind;
use std::time::Duration;
use tokio::sync::{oneshot, watch};
use veila_common::PowerAction;
const DBUS_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) async fn arm(output: &mut Output, generation: u64) -> bool {
    let (ack, response) = oneshot::channel();
    output.send(Event::Arm { generation, ack }).await
        && matches!(
            tokio::time::timeout(DBUS_TIMEOUT, response).await,
            Ok(Ok(()))
        )
}
pub(super) async fn perform(
    action: PowerAction,
    manager: &logind::ManagerProxy<'_>,
    generation: u64,
    output: &mut Output,
    changes: &watch::Receiver<State>,
    automatic: bool,
    inhibitor: &mut Option<zbus::OwnedFd>,
) {
    if action == PowerAction::Suspend && !arm(output, generation).await {
        tracing::error!("suspend cancelled: resume guard was not acknowledged");
        return;
    }
    let state = changes.borrow().clone();
    if state.generation != generation
        || !state.confirmed
        || if automatic {
            state.auth || !state.config.lock.suspend_seconds.is_some_and(|s| s > 0)
        } else {
            !state.config.visuals.power_button_enabled(action)
        }
    {
        return;
    }
    if action == PowerAction::Suspend {
        // Our guard is armed; do not make logind wait for our own delay inhibitor.
        *inhibitor = None;
    }
    let request = crate::services::power::request_power_action(manager, action);
    match tokio::time::timeout(DBUS_TIMEOUT, request).await {
        Ok(Ok(())) => tracing::info!(?action, "supervisor power request accepted"),
        result => tracing::warn!(
            ?action,
            ?result,
            "supervisor power request failed or timed out"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    #[tokio::test(start_paused = true)]
    async fn suspend_requires_an_acknowledgement_from_the_curtain() {
        let (tx, mut rx) = mpsc::channel(2);
        let (wake, _reader) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut output = Output { tx, wake };
        let task = tokio::spawn(async move { arm(&mut output, 7).await });
        match rx.recv().await.unwrap() {
            Event::Arm { generation, ack } => {
                assert_eq!(generation, 7);
                assert!(!task.is_finished());
                ack.send(()).unwrap();
            }
            _ => panic!("expected guard request"),
        }
        assert!(task.await.unwrap());
    }
    #[tokio::test(start_paused = true)]
    async fn missing_guard_acknowledgement_times_out_without_suspend_permission() {
        let (tx, mut rx) = mpsc::channel(2);
        let (wake, _reader) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut output = Output { tx, wake };
        let task = tokio::spawn(async move { arm(&mut output, 3).await });
        let Event::Arm { ack, .. } = rx.recv().await.unwrap() else {
            panic!("guard")
        };
        tokio::time::advance(DBUS_TIMEOUT + Duration::from_millis(1)).await;
        assert!(!task.await.unwrap());
        assert!(ack.send(()).is_err());
    }
}
