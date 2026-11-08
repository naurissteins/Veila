use crate::record::{self, Record, State};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub struct ProcessStatus {
    pub pid: u32,
    pub start_ticks: u64,
    pub config: Option<PathBuf>,
}
#[derive(Debug, Default)]
pub struct RuntimeStatus {
    pub state: &'static str,
    pub supervisor: Option<ProcessStatus>,
    pub curtain: Option<u32>,
    pub idle: Option<ProcessStatus>,
}

#[derive(Serialize, Deserialize)]
struct WatcherIdentity {
    pid: u32,
    start_ticks: u64,
    config: Option<PathBuf>,
}

pub(crate) struct WatcherRecord(File);
impl WatcherRecord {
    pub(crate) fn acquire(config: Option<PathBuf>) -> Result<Self> {
        let (runtime, display) = record::environment()?;
        let mut file = record::open_record(runtime, display, "idle")?;
        if !record::acquire_exclusive(&file)? {
            bail!("an idle watcher is already running for this display");
        }
        let bytes = serde_json::to_vec(&WatcherIdentity {
            pid: std::process::id(),
            start_ticks: record::start_ticks(std::process::id())?,
            config,
        })?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&bytes)?;
        file.set_len(bytes.len() as u64)?;
        Ok(Self(file))
    }
}
impl Drop for WatcherRecord {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn matches(pid: u32, ticks: u64) -> bool {
    pid > 0 && record::start_ticks(pid).is_ok_and(|actual| actual == ticks)
}

pub fn runtime_status() -> Result<RuntimeStatus> {
    let (runtime, display) = record::environment()?;
    status_at(runtime, display)
}

fn status_at(runtime: PathBuf, display: PathBuf) -> Result<RuntimeStatus> {
    let mut status = RuntimeStatus {
        state: "unlocked",
        ..Default::default()
    };
    if let Some(bytes) = record::held_bytes(record::LockRecord::open_at(
        runtime.clone(),
        display.clone(),
    )?)? {
        status.state = "unknown";
        if let Ok(value) = serde_json::from_slice::<Record>(&bytes) {
            status.state = match value.state {
                State::Starting => "starting",
                State::Locked => "locked",
                State::Unlocked => "unlocked",
                State::Refused => "refused",
                State::Abnormal => "abnormal",
            };
            if matches(value.supervisor_pid, value.supervisor_start_ticks) {
                status.supervisor = Some(ProcessStatus {
                    pid: value.supervisor_pid,
                    start_ticks: value.supervisor_start_ticks,
                    config: value.config,
                });
            }
            if let (Some(pid), Some(ticks)) = (value.curtain_pid, value.curtain_start_ticks)
                && matches(pid, ticks)
            {
                status.curtain = Some(pid);
            }
        }
    }
    if let Some(bytes) = record::held_bytes(record::open_record(runtime, display, "idle")?)? {
        let value: WatcherIdentity =
            serde_json::from_slice(&bytes).context("idle watcher record is incomplete")?;
        if matches(value.pid, value.start_ticks) {
            status.idle = Some(ProcessStatus {
                pid: value.pid,
                start_ticks: value.start_ticks,
                config: value.config,
            });
        }
    }
    Ok(status)
}

pub fn reload() -> Result<usize> {
    let status = runtime_status()?;
    let mut count = 0;
    for process in [status.supervisor, status.idle].into_iter().flatten() {
        // Pin the target before checking start time so PID reuse cannot redirect the signal.
        let target = veila_common::session::process::Target::open(process.pid)?;
        if !matches(process.pid, process.start_ticks) {
            bail!("process identity changed; reload was not sent");
        }
        target.hangup()?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests;
