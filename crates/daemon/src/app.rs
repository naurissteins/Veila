use std::{future::pending, process::ExitStatus};

use anyhow::{Context, Result, anyhow};
use futures_util::StreamExt;
use tokio::{
    net::UnixListener,
    process::Child,
    signal::unix::{SignalKind, signal},
    time::{Duration, timeout},
};

use crate::{
    adapters::{logind, process},
    domain::lock_state::LockState,
};

pub async fn run() -> Result<()> {
    let connection = logind::connect_system().await?;
    let session_path = logind::get_session_path(&connection).await?;
    let session_proxy = logind::session_proxy(&connection, &session_path).await?;
    let mut lock_stream = session_proxy
        .receive_lock()
        .await
        .context("failed to subscribe to logind Lock signal")?;
    let mut unlock_stream = session_proxy
        .receive_unlock()
        .await
        .context("failed to subscribe to logind Unlock signal")?;
    let mut sigint =
        signal(SignalKind::interrupt()).context("failed to register SIGINT handler")?;
    let mut sigterm =
        signal(SignalKind::terminate()).context("failed to register SIGTERM handler")?;

    let mut state = LockState::Unlocked;
    let mut curtain: Option<Child> = None;

    tracing::info!(session = %session_path, "kwylockd ready");

    loop {
        tokio::select! {
            Some(_) = lock_stream.next() => {
                if state.is_active() {
                    tracing::debug!(state = %state, "ignoring duplicate lock signal");
                    continue;
                }

                if let Err(error) = activate_lock(&session_proxy, &mut state, &mut curtain).await {
                    tracing::error!("failed to activate lock: {error:#}");
                }
            }
            Some(_) = unlock_stream.next() => {
                if !state.is_active() {
                    tracing::debug!(state = %state, "ignoring unlock signal while not locked");
                    continue;
                }

                if let Err(error) = deactivate_lock(&session_proxy, &mut state, &mut curtain).await {
                    tracing::error!("failed to deactivate lock: {error:#}");
                }
            }
            result = wait_for_curtain_exit(&mut curtain), if curtain.is_some() => {
                let status = result?;
                tracing::warn!(?status, state = %state, "curtain exited");
                curtain.take();

                if state.is_active() {
                    update_locked_hint(&session_proxy, false).await;
                    state = LockState::Unlocked;
                    tracing::error!("curtain exited while the session should be locked; attempting restart");

                    if let Err(error) = activate_lock(&session_proxy, &mut state, &mut curtain).await {
                        tracing::error!("failed to restart curtain after unexpected exit: {error:#}");
                    }
                }
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT");
                break;
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
                break;
            }
        }
    }

    if let Err(error) = deactivate_lock(&session_proxy, &mut state, &mut curtain).await {
        tracing::warn!("failed to stop curtain during shutdown: {error:#}");
    }

    tracing::info!("kwylockd exiting");
    Ok(())
}

async fn activate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    curtain: &mut Option<Child>,
) -> Result<()> {
    *state = LockState::Locking;

    let notify_path = process::notify_socket_path();
    let _ = std::fs::remove_file(&notify_path);
    let listener = UnixListener::bind(&notify_path).with_context(|| {
        format!(
            "failed to bind curtain notify socket {}",
            notify_path.display()
        )
    })?;
    let child = process::spawn_curtain(&notify_path).await?;
    *curtain = Some(child);

    let ready_result = timeout(Duration::from_secs(5), listener.accept()).await;
    let _ = std::fs::remove_file(&notify_path);

    match ready_result {
        Ok(Ok((_stream, _addr))) => {
            *state = LockState::Locked;
            update_locked_hint(session_proxy, true).await;
            tracing::info!("curtain ready; session considered locked");
            Ok(())
        }
        Ok(Err(error)) => {
            *state = LockState::Unlocked;
            stop_curtain(curtain).await?;
            update_locked_hint(session_proxy, false).await;
            Err(error).context("failed while waiting for curtain readiness")
        }
        Err(_) => {
            *state = LockState::Unlocked;
            stop_curtain(curtain).await?;
            update_locked_hint(session_proxy, false).await;
            Err(anyhow!("timed out waiting for curtain readiness"))
        }
    }
}

async fn deactivate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    curtain: &mut Option<Child>,
) -> Result<()> {
    if curtain.is_none() {
        *state = LockState::Unlocked;
        update_locked_hint(session_proxy, false).await;
        return Ok(());
    }

    *state = LockState::Unlocking;
    stop_curtain(curtain).await?;
    *state = LockState::Unlocked;
    update_locked_hint(session_proxy, false).await;

    tracing::info!("curtain stopped; session considered unlocked");
    Ok(())
}

async fn stop_curtain(curtain: &mut Option<Child>) -> Result<()> {
    if let Some(child) = curtain.take() {
        process::stop_curtain(child).await?;
    }

    Ok(())
}

async fn wait_for_curtain_exit(curtain: &mut Option<Child>) -> Result<ExitStatus> {
    match curtain.as_mut() {
        Some(child) => child
            .wait()
            .await
            .context("failed while waiting for curtain process"),
        None => pending().await,
    }
}

async fn update_locked_hint(session_proxy: &logind::SessionProxy<'_>, locked: bool) {
    if let Err(error) = session_proxy.set_locked_hint(locked).await {
        tracing::warn!(locked, "failed to update logind LockedHint: {error}");
    }
}
