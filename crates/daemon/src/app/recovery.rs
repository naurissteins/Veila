use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use tokio::sync::mpsc::unbounded_channel;
use veila_common::ipc::CurtainLockState;

use crate::{
    adapters::{ipc, ownership, process::CurtainHandle},
    domain::{auth::AuthState, lock_state::LockState},
};

use super::{runtime::ActiveLock, state::AppRuntime};

pub(super) async fn adopt_surviving_curtain(session: &str, runtime: &mut AppRuntime) -> Result<()> {
    let path = ownership::record_path(session)?;
    ownership::reject_other_session_records(&path)?;
    let Some(record) = ownership::load(&path, session)? else {
        return Ok(());
    };
    let Some((pid, start_ticks)) = record.pid.zip(record.start_ticks) else {
        // startup gate cannot open before a process identity is published
        ownership::remove(&path)?;
        tracing::warn!("removed incomplete curtain ownership record from interrupted spawn");
        return Ok(());
    };
    if record.gate_was_closed() {
        for _ in 0..10 {
            if !ownership::process_matches(pid, start_ticks)? {
                ownership::remove(&path)?;
                tracing::warn!(pid, "removed interrupted curtain spawn before gate release");
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        bail!("curtain {pid} did not exit after its closed startup gate lost the daemon");
    }
    if !ownership::process_matches(pid, start_ticks)? {
        bail!(
            "curtain ownership is unresolved: process {pid} is absent or its identity changed; refusing a competing lock"
        );
    }

    let deadline = Instant::now()
        + Duration::from_secs(
            runtime
                .loaded_config
                .config
                .lock
                .acquire_timeout_seconds
                .max(1)
                + 2,
        );
    let mut last_error = None;
    loop {
        if !ownership::process_matches(pid, start_ticks)? {
            bail!("curtain {pid} exited during adoption; lock ownership is unresolved");
        }
        match crate::adapters::process::probe_curtain(&record.control_socket, pid).await {
            Ok(CurtainLockState::Locked) => break,
            Ok(CurtainLockState::Finished) => {
                bail!("curtain {pid} reports its session lock was revoked; refusing adoption");
            }
            Ok(CurtainLockState::Starting) => {}
            Err(error) => last_error = Some(error),
        }
        if Instant::now() >= deadline {
            let detail = last_error
                .map(|error| format!(": {error:#}"))
                .unwrap_or_default();
            bail!("curtain {pid} did not confirm a lock before the adoption deadline{detail}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let listener = ipc::bind_listener(&record.auth_socket)
        .await
        .context("failed to restore the orphan curtain authentication socket")?;
    if !ownership::process_matches(pid, start_ticks)? {
        let _ = std::fs::remove_file(&record.auth_socket);
        bail!("curtain {pid} exited as authentication was restored");
    }
    let (auth_sender, auth_results) = unbounded_channel();
    runtime.active = Some(ActiveLock {
        curtain: CurtainHandle::Adopted {
            pid,
            start_ticks,
            owner_path: path,
            next_check: tokio::time::Instant::now(),
        },
        auth_listener: listener,
        auth_socket_path: record.auth_socket,
        control_socket_path: record.control_socket,
        auth_sender,
        auth_results,
    });
    runtime.auth_state = AuthState::after_daemon_recovery(runtime.auth_policy, Instant::now());
    runtime.state = LockState::Locked;
    runtime.suspend_state.arm(Instant::now());
    tracing::warn!(pid, "adopted surviving locked curtain after daemon restart");
    Ok(())
}
