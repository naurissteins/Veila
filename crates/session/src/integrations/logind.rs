use super::actions::{DBUS_TIMEOUT, arm, inhibit, perform};
use super::{Output, Request, State};
use crate::adapters::logind;
use futures_util::StreamExt;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use veila_common::{PowerAction, ipc::CurtainControlMessage, session::SupervisorMessage};

pub(super) async fn run(
    mut changes: watch::Receiver<State>,
    mut requests: mpsc::Receiver<Request>,
    mut output: Output,
    fingerprint: super::fingerprint::Fingerprint,
    awake_since: watch::Sender<Option<Instant>>,
) {
    loop {
        if let Err(error) = serve(
            &mut changes,
            &mut requests,
            &mut output,
            &fingerprint,
            &awake_since,
        )
        .await
        {
            awake_since.send_replace(None);
            fingerprint.pause().await;
            tracing::warn!(error=%format!("{error:#}"), "logind integration unavailable; retaining local PAM and retrying");
        } else {
            return;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                changed = changes.changed() => { if changed.is_err() { fingerprint.stop().await; return; } }
                request = requests.recv() => {
                    if let Some(Request::Finish(ack)) = request { fingerprint.stop().await; let _ = ack.send(()); return; }
                }
            }
        }
    }
}

async fn serve(
    changes: &mut watch::Receiver<State>,
    requests: &mut mpsc::Receiver<Request>,
    output: &mut Output,
    fingerprint: &super::fingerprint::Fingerprint,
    awake_since: &watch::Sender<Option<Instant>>,
) -> anyhow::Result<()> {
    let connection = tokio::time::timeout(DBUS_TIMEOUT, logind::connect_system()).await??;
    let path =
        tokio::time::timeout(DBUS_TIMEOUT, logind::get_session_path(&connection, None)).await??;
    let session = logind::session_proxy(&connection, &path).await?;
    let manager = logind::ManagerProxy::new(&connection).await?;
    let mut owner = manager.inner().receive_owner_changed().await?;
    let mut unlock = session.receive_unlock().await?;
    let mut sleep = manager.receive_prepare_for_sleep().await?;
    // A newly spawned supervisor may have missed the beginning of sleep preparation.
    let preparing = tokio::time::timeout(DBUS_TIMEOUT, manager.preparing_for_sleep()).await;
    let sleeping = matches!(preparing, Ok(Ok(true)));
    match preparing {
        Ok(Ok(false)) => fingerprint.resume().await,
        Ok(Ok(true)) => {}
        result => tracing::warn!(
            ?result,
            "sleep state unavailable; fingerprint remains paused"
        ),
    }
    let mut inhibitor = inhibit(&manager).await;
    let mut marked = changes.borrow().confirmed;
    if marked {
        hint(&session, true).await;
    }
    tracing::info!(session = %path, "supervisor logind integration ready");
    if !sleeping {
        awake_since.send_replace(Some(Instant::now()));
    }
    loop {
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() { return Ok(()); }
                let confirmed = changes.borrow_and_update().confirmed;
                if confirmed && !marked { hint(&session, true).await; marked = true; }
            }
            _ = owner.next() => anyhow::bail!("logind service owner changed"),
            signal = unlock.next() => {
                let Some(_) = signal else { anyhow::bail!("logind Unlock stream closed"); };
                let state = changes.borrow().clone();
                if state.confirmed {
                    tracing::info!(generation=state.generation, "logind authorized session unlock");
                    fingerprint.pause().await;
                    output.message(state.generation, SupervisorMessage::Control(CurtainControlMessage::Unlock { attempt_id: None })).await;
                }
            }
            signal = sleep.next() => {
                let Some(signal) = signal else { anyhow::bail!("logind sleep stream closed"); };
                let start = *signal.args()?.start();
                let state = changes.borrow().clone();
                if start {
                    awake_since.send_replace(None);
                    fingerprint.pause().await;
                    if state.confirmed && !arm(output, state.generation).await {
                        tracing::error!("resume guard acknowledgement unavailable before sleep; lock remains held");
                    }
                    inhibitor = None;
                } else {
                    let resumed_at = Instant::now();
                    fingerprint.resume().await;
                    output.message(state.generation, SupervisorMessage::Control(CurtainControlMessage::MarkResumed)).await;
                    inhibitor = inhibit(&manager).await;
                    awake_since.send_replace(Some(resumed_at));
                }
            }
            request = requests.recv() => {
                match request {
                    Some(Request::Finish(ack)) => { fingerprint.stop().await; hint(&session, false).await; let _ = ack.send(()); return Ok(()); }
                    Some(Request::Power { generation, action }) => {
                        if perform(action, &manager, generation, output, changes, None, &mut inhibitor).await {
                            awake_since.send_replace(None);
                        }
                    }
                    Some(Request::Suspend { attempt, done }) => {
                        let current = *awake_since.borrow() == Some(attempt.awake_since);
                        if current && perform(
                            PowerAction::Suspend,
                            &manager,
                            attempt.generation,
                            output,
                            changes,
                            Some(attempt),
                            &mut inhibitor,
                        ).await {
                            awake_since.send_replace(None);
                        }
                        let _ = done.send(());
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
