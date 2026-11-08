use crate::{
    LockOptions,
    record::{self, LockRecord, Record, State},
};
use anyhow::{Result, bail};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) enum Acquisition {
    Owner(LockRecord),
    Duplicate { locked: bool },
}

enum Inspection {
    Vacant,
    Pending,
    Known(Record),
}

pub(crate) fn acquire(options: &LockOptions) -> Result<Acquisition> {
    let (runtime, display) = record::environment()?;
    acquire_at(
        runtime,
        display,
        options.config.clone(),
        options.wait_ready || options.background_fd.is_some(),
        Duration::from_secs(120),
    )
}

fn acquire_at(
    runtime: PathBuf,
    display: PathBuf,
    config: Option<PathBuf>,
    wait_ready: bool,
    timeout: Duration,
) -> Result<Acquisition> {
    let deadline = Instant::now() + timeout;
    let mut following = false;
    loop {
        if !following
            && let Some(record) = LockRecord::at(runtime.clone(), display.clone(), config.clone())?
        {
            return Ok(Acquisition::Owner(record));
        }
        let inspection = inspect(runtime.clone(), display.clone())?;
        match inspection {
            Inspection::Vacant if following => {
                bail!("active lock ended before readiness was confirmed")
            }
            Inspection::Vacant => {}
            Inspection::Known(record)
                if matches!(
                    record.state,
                    State::Unlocked | State::Refused | State::Abnormal
                ) =>
            {
                if following {
                    bail!("active lock ended before readiness was confirmed");
                }
                // Closing owners finish their bounded cleanup before a new lock takes over.
            }
            Inspection::Known(record) => {
                if wait_ready && record.ready.is_none() {
                    bail!(
                        "active supervisor predates readiness records; unlock normally before retrying"
                    );
                }
                let locked = record.state == State::Locked && curtain_alive(&record);
                if locked && (!wait_ready || record.ready == Some(true)) {
                    return Ok(Acquisition::Duplicate { locked: true });
                }
                if !wait_ready {
                    return Ok(Acquisition::Duplicate { locked: false });
                }
                following = true;
            }
            Inspection::Pending => {
                if !wait_ready {
                    return Ok(Acquisition::Duplicate { locked: false });
                }
                following = true;
            }
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for the active lock's readiness");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn inspect(runtime: PathBuf, display: PathBuf) -> Result<Inspection> {
    let Some(bytes) = record::held_bytes(LockRecord::open_at(runtime, display)?)? else {
        return Ok(Inspection::Vacant);
    };
    Ok(serde_json::from_slice(&bytes).map_or(Inspection::Pending, Inspection::Known))
}

fn curtain_alive(record: &Record) -> bool {
    match (record.curtain_pid, record.curtain_start_ticks) {
        (Some(pid), Some(ticks)) => record::start_ticks(pid).is_ok_and(|actual| actual == ticks),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
