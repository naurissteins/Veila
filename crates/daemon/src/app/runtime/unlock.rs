use std::{path::Path, time::Instant};

use anyhow::{Result, anyhow};
use tokio::time::{Duration, timeout};
use veila_common::elapsed_ms;

use crate::{
    adapters::{
        logind, ownership,
        process::{self, CurtainHandle},
    },
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};

use super::{
    active::ActiveLock,
    state::{reset_runtime, update_locked_hint},
};

pub(crate) async fn deactivate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    runtime: &mut Option<ActiveLock>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
    attempt_id: Option<u64>,
) -> Result<()> {
    let started_at = Instant::now();
    if runtime.is_none() {
        if state.is_active() {
            return Err(anyhow!(
                "curtain ownership is unresolved; refusing to clear the lock state"
            ));
        }
        *state = LockState::Unlocked;
        reset_runtime(runtime, auth_policy, auth_state);
        update_locked_hint(session_proxy, false).await;
        tracing::info!(
            elapsed_ms = elapsed_ms(started_at),
            "deactivate_lock completed without active curtain"
        );
        return Ok(());
    }

    *state = LockState::Unlocking;

    if let Some(active) = runtime.as_mut() {
        let owner_path = active.curtain.owner_path().to_path_buf();
        match stop_active_curtain(&mut active.curtain, &active.control_socket_path, attempt_id)
            .await
        {
            CurtainStop::Released => {
                if let Err(error) = ownership::remove(&owner_path) {
                    tracing::warn!("failed to clear released curtain ownership: {error:#}");
                }
            }
            CurtainStop::UnlockUndeliverable => {
                *state = LockState::Locked;
                return Err(anyhow!(
                    "could not deliver the unlock to the curtain; session stays locked"
                ));
            }
        }
    }

    reset_runtime(runtime, auth_policy, auth_state);
    *state = LockState::Unlocked;
    update_locked_hint(session_proxy, false).await;

    let elapsed_ms = elapsed_ms(started_at);
    if let Some(attempt_id) = attempt_id {
        tracing::info!(
            attempt_id,
            elapsed_ms,
            "curtain stopped; session considered unlocked"
        );
    } else {
        tracing::info!(elapsed_ms, "curtain stopped; session considered unlocked");
    }
    Ok(())
}

pub(crate) enum CurtainStop {
    /// The curtain was told to unlock and is no longer holding the session lock.
    Released,
    /// Release was not confirmed, so the curtain ownership must be retained.
    UnlockUndeliverable,
}

pub(in crate::app) const GRACEFUL_EXIT_WINDOW: Duration = Duration::from_secs(5);
const UNLOCK_DELIVERY_ATTEMPTS: u32 = 3;
const UNLOCK_RETRY_DELAY: Duration = Duration::from_millis(100);

async fn stop_active_curtain(
    child: &mut CurtainHandle,
    control_socket_path: &Path,
    attempt_id: Option<u64>,
) -> CurtainStop {
    if let Err(error) = deliver_unlock(control_socket_path, attempt_id).await {
        tracing::error!("could not deliver unlock to the curtain: {error:#}");
        return CurtainStop::UnlockUndeliverable;
    }

    match timeout(GRACEFUL_EXIT_WINDOW, child.wait()).await {
        Ok(Ok(_)) => return CurtainStop::Released,
        Ok(Err(error)) => {
            tracing::error!("failed while waiting for curtain to exit: {error:#}");
            return CurtainStop::UnlockUndeliverable;
        }
        Err(_) => {}
    }
    if matches!(child, CurtainHandle::Adopted { .. }) {
        tracing::error!("adopted curtain did not exit after unlock delivery; preserving ownership");
        return CurtainStop::UnlockUndeliverable;
    }
    tracing::warn!("curtain did not exit after unlock delivery; requesting termination");
    if let Err(error) = child
        .signal_if_running(nix::sys::signal::Signal::SIGTERM)
        .await
    {
        tracing::error!("failed to signal curtain after unlock delivery: {error:#}");
        return CurtainStop::UnlockUndeliverable;
    }
    match timeout(Duration::from_secs(2), child.wait()).await {
        Ok(Ok(_)) => CurtainStop::Released,
        Ok(Err(error)) => {
            tracing::error!("failed while waiting for curtain to exit: {error:#}");
            CurtainStop::UnlockUndeliverable
        }
        Err(_) => CurtainStop::UnlockUndeliverable,
    }
}

pub(in crate::app) async fn deliver_unlock(
    control_socket_path: &Path,
    attempt_id: Option<u64>,
) -> Result<()> {
    let mut last_error = None;

    for attempt in 1..=UNLOCK_DELIVERY_ATTEMPTS {
        match process::request_curtain_unlock(control_socket_path, attempt_id).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                tracing::warn!(attempt, "failed to request curtain unlock: {error:#}");
                last_error = Some(error);
                if attempt < UNLOCK_DELIVERY_ATTEMPTS {
                    tokio::time::sleep(UNLOCK_RETRY_DELAY).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("unlock delivery failed")))
}

#[cfg(test)]
mod tests;
