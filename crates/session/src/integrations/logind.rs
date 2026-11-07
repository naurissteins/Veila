use super::actions::{arm, perform};
use super::{Output, Request, State};
use crate::adapters::logind;
use futures_util::StreamExt;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use veila_common::{PowerAction, ipc::CurtainControlMessage, session::SupervisorMessage};

const DBUS_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) async fn run(
    mut changes: watch::Receiver<State>,
    mut requests: mpsc::Receiver<Request>,
    mut output: Output,
) {
    loop {
        if let Err(error) = serve(&mut changes, &mut requests, &mut output).await {
            tracing::warn!(error=%format!("{error:#}"), "logind integration unavailable; retaining local PAM and retrying");
        } else {
            return;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                changed = changes.changed() => { if changed.is_err() { return; } }
                request = requests.recv() => {
                    if let Some(Request::Finish(ack)) = request { let _ = ack.send(()); return; }
                }
            }
        }
    }
}

async fn serve(
    changes: &mut watch::Receiver<State>,
    requests: &mut mpsc::Receiver<Request>,
    output: &mut Output,
) -> anyhow::Result<()> {
    let connection = tokio::time::timeout(DBUS_TIMEOUT, logind::connect_system()).await??;
    let path =
        tokio::time::timeout(DBUS_TIMEOUT, logind::get_session_path(&connection, None)).await??;
    let session = logind::session_proxy(&connection, &path).await?;
    let manager = logind::ManagerProxy::new(&connection).await?;
    let mut unlock = session.receive_unlock().await?;
    let mut sleep = manager.receive_prepare_for_sleep().await?;
    let mut inhibitor = inhibit(&manager).await;
    let mut marked = changes.borrow().confirmed;
    if marked {
        hint(&session, true).await;
    }
    tracing::info!(session = %path, "supervisor logind integration ready");
    loop {
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() { return Ok(()); }
                let confirmed = changes.borrow_and_update().confirmed;
                if confirmed && !marked { hint(&session, true).await; marked = true; }
            }
            signal = unlock.next() => {
                let Some(_) = signal else { anyhow::bail!("logind Unlock stream closed"); };
                let state = changes.borrow().clone();
                if state.confirmed {
                    tracing::info!(generation=state.generation, "logind authorized session unlock");
                    output.message(state.generation, SupervisorMessage::Control(CurtainControlMessage::Unlock { attempt_id: None })).await;
                }
            }
            signal = sleep.next() => {
                let Some(signal) = signal else { anyhow::bail!("logind sleep stream closed"); };
                let start = *signal.args()?.start();
                let state = changes.borrow().clone();
                if start {
                    if state.confirmed && !arm(output, state.generation).await {
                        tracing::error!("resume guard acknowledgement unavailable before sleep; lock remains held");
                    }
                    inhibitor = None;
                } else {
                    output.message(state.generation, SupervisorMessage::Control(CurtainControlMessage::MarkResumed)).await;
                    inhibitor = inhibit(&manager).await;
                }
            }
            request = requests.recv() => {
                match request {
                    Some(Request::Finish(ack)) => { hint(&session, false).await; let _ = ack.send(()); return Ok(()); }
                    Some(Request::Power { generation, action }) => {
                        let state = changes.borrow().clone();
                        if state.generation == generation && state.confirmed && state.config.visuals.power_button_enabled(action) {
                            perform(action, &manager, generation, output, changes, false, &mut inhibitor).await;
                        }
                    }
                    Some(Request::Suspend { generation }) => {
                        let state = changes.borrow().clone();
                        if state.generation == generation && state.confirmed && !state.auth && state.config.lock.suspend_seconds.is_some_and(|s| s>0) {
                            perform(PowerAction::Suspend, &manager, generation, output, changes, true, &mut inhibitor).await;
                        }
                    }
                    None => return Ok(()),
                }
            }
        }
        let _ = &inhibitor;
    }
}

async fn hint(session: &logind::SessionProxy<'_>, locked: bool) {
    if !matches!(
        tokio::time::timeout(DBUS_TIMEOUT, session.set_locked_hint(locked)).await,
        Ok(Ok(()))
    ) {
        tracing::warn!(locked, "could not update best-effort LockedHint");
    }
}
async fn inhibit(manager: &logind::ManagerProxy<'_>) -> Option<zbus::OwnedFd> {
    match tokio::time::timeout(
        DBUS_TIMEOUT,
        manager.inhibit("sleep", "Veila", "Arm locked resume input guard", "delay"),
    )
    .await
    {
        Ok(Ok(fd)) => Some(fd),
        _ => {
            tracing::warn!("could not acquire resume-guard sleep delay inhibitor");
            None
        }
    }
}
