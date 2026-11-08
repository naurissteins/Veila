use super::Trigger;
use crate::adapters::logind;
use anyhow::{Result, bail};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::{Instant, timeout},
};
use veila_common::IdleConfig;

const CALL_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) async fn run(
    mut settings: watch::Receiver<IdleConfig>,
    triggers: mpsc::Sender<Trigger>,
    session_id: Option<String>,
) {
    loop {
        let result = serve(&mut settings, &triggers, session_id.as_deref()).await;
        if triggers.is_closed() {
            return;
        }
        tracing::warn!(error=?result.err(), "idle watcher logind unavailable; retrying");
        tokio::select! {
            () = triggers.closed() => return,
            () = tokio::time::sleep(Duration::from_secs(5)) => {},
        }
    }
}

async fn serve(
    settings: &mut watch::Receiver<IdleConfig>,
    triggers: &mpsc::Sender<Trigger>,
    session_id: Option<&str>,
) -> Result<()> {
    let connection = timeout(CALL_TIMEOUT, logind::connect_system()).await??;
    let path = timeout(
        CALL_TIMEOUT,
        logind::get_session_path(&connection, session_id),
    )
    .await??;
    let session = logind::session_proxy(&connection, &path).await?;
    let manager = logind::ManagerProxy::new(&connection).await?;
    let mut locks = session.receive_lock().await?;
    let mut sleeps = manager.receive_prepare_for_sleep().await?;
    let budget = match timeout(CALL_TIMEOUT, manager.inhibit_delay_max_usec()).await {
        Ok(Ok(micros)) => Duration::from_micros(micros).min(Duration::from_secs(120)),
        _ => Duration::ZERO,
    };
    let mut sleeping = timeout(CALL_TIMEOUT, manager.preparing_for_sleep()).await??;
    let mut inhibitor = None;
    let mut confirmation = None;
    let mut deadline = None;
    let enabled = settings.borrow().lock_before_sleep;
    sync(&manager, enabled, sleeping, &mut inhibitor).await;
    if sleeping && settings.borrow().lock_before_sleep {
        begin(triggers, budget, &mut confirmation, &mut deadline).await?;
    }
    tracing::info!(session=%path, "idle watcher logind ready");
    loop {
        tokio::select! {
            () = triggers.closed() => return Ok(()),
            change = settings.changed() => {
                change?;
                let enabled = settings.borrow_and_update().lock_before_sleep;
                sync(&manager, enabled, sleeping, &mut inhibitor).await;
            }
            signal = locks.next() => {
                if signal.is_none() { bail!("logind Lock stream closed"); }
                triggers.send(Trigger::Lock).await?;
            }
            signal = sleeps.next() => {
                let Some(signal) = signal else { bail!("logind sleep stream closed"); };
                sleeping = *signal.args()?.start();
                if sleeping && settings.borrow().lock_before_sleep {
                    begin(triggers, budget, &mut confirmation, &mut deadline).await?;
                } else {
                    confirmation = None;
                    deadline = None;
                    let enabled = settings.borrow().lock_before_sleep;
                    sync(&manager, enabled, sleeping, &mut inhibitor).await;
                }
            }
            confirmed = async { match &mut confirmation { Some(rx) => rx.await.is_ok(), None => std::future::pending().await } } => {
                tracing::info!(confirmed, "idle watcher releasing sleep inhibitor");
                inhibitor = None;
                confirmation = None;
                deadline = None;
            }
            () = async { match deadline { Some(at) => tokio::time::sleep_until(at).await, None => std::future::pending().await } } => {
                tracing::warn!("lock confirmation exceeded logind delay; releasing sleep inhibitor");
                inhibitor = None;
                confirmation = None;
                deadline = None;
            }
        }
    }
}

async fn begin(
    triggers: &mpsc::Sender<Trigger>,
    budget: Duration,
    confirmation: &mut Option<oneshot::Receiver<()>>,
    deadline: &mut Option<Instant>,
) -> Result<()> {
    if confirmation.is_some() {
        return Ok(());
    }
    let (tx, rx) = oneshot::channel();
    // Reserve time for logind to process the close before its hard delay expires.
    *deadline = Some(Instant::now() + budget.saturating_sub(Duration::from_millis(100)));
    *confirmation = Some(rx);
    triggers.send(Trigger::Sleep(tx)).await?;
    Ok(())
}

async fn sync(
    manager: &logind::ManagerProxy<'_>,
    enabled: bool,
    sleeping: bool,
    inhibitor: &mut Option<zbus::OwnedFd>,
) {
    if !enabled {
        *inhibitor = None;
        return;
    }
    if sleeping || inhibitor.is_some() {
        return;
    }
    match timeout(
        CALL_TIMEOUT,
        manager.inhibit(
            "sleep",
            "Veila idle",
            "Lock the session before sleep",
            "delay",
        ),
    )
    .await
    {
        Ok(Ok(fd)) => {
            *inhibitor = Some(fd);
            tracing::info!("idle watcher sleep inhibitor armed");
        }
        result => tracing::warn!(?result, "could not arm lock-before-sleep inhibitor"),
    }
}
