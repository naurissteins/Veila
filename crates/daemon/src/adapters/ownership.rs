use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use nix::unistd::Uid;
use serde::{Deserialize, Serialize};

use super::ipc;

const RECORD_VERSION: u8 = 1;
const MAX_RECORD_BYTES: u64 = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OwnerRecord {
    version: u8,
    pub(crate) session: String,
    pub(crate) pid: Option<u32>,
    pub(crate) start_ticks: Option<u64>,
    gate_may_open: bool,
    pub(crate) auth_socket: PathBuf,
    pub(crate) control_socket: PathBuf,
}

impl OwnerRecord {
    pub(crate) fn pending(session: &str, auth_socket: PathBuf, control_socket: PathBuf) -> Self {
        Self {
            version: RECORD_VERSION,
            session: session.to_owned(),
            pid: None,
            start_ticks: None,
            gate_may_open: false,
            auth_socket,
            control_socket,
        }
    }

    pub(crate) fn set_process(&mut self, pid: u32) -> Result<()> {
        self.pid = Some(pid);
        self.start_ticks = Some(process_start_ticks(pid)?);
        Ok(())
    }

    pub(crate) fn mark_gate_opening(&mut self) {
        self.gate_may_open = true;
    }

    pub(crate) fn gate_was_closed(&self) -> bool {
        !self.gate_may_open
    }

    fn validate(&self, session: &str, runtime_dir: &Path) -> Result<()> {
        if self.version != RECORD_VERSION || self.session != session {
            bail!("curtain ownership record has an incompatible version or session");
        }
        if self.pid.is_some() != self.start_ticks.is_some() {
            bail!("curtain ownership record has an incomplete process identity");
        }
        if self.gate_may_open && self.pid.is_none() {
            bail!("curtain ownership record opened the gate without a process identity");
        }
        if self.auth_socket.parent() != Some(runtime_dir)
            || self.control_socket.parent() != Some(runtime_dir)
        {
            bail!("curtain ownership record names sockets outside the runtime directory");
        }
        Ok(())
    }
}

pub(crate) fn record_path(session: &str) -> Result<PathBuf> {
    let segment = session
        .rsplit('/')
        .next()
        .filter(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
        .ok_or_else(|| anyhow!("invalid logind session path for curtain ownership"))?;
    Ok(ipc::runtime_dir()?.join(format!("curtain-owner-{segment}.json")))
}

pub(crate) fn reject_other_session_records(expected: &Path) -> Result<()> {
    let directory = expected
        .parent()
        .context("ownership record has no parent directory")?;
    for entry in fs::read_dir(directory).context("failed to scan curtain ownership records")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("curtain-owner-") && name.ends_with(".json") && entry.path() != expected
        {
            bail!("curtain ownership for another logind session is unresolved");
        }
    }
    Ok(())
}

pub(crate) fn load(path: &Path, session: &str) -> Result<Option<OwnerRecord>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("failed to inspect curtain ownership record"),
    };
    if !metadata.file_type().is_file()
        || metadata.uid() != Uid::effective().as_raw()
        || metadata.len() > MAX_RECORD_BYTES
    {
        bail!("curtain ownership record is not a small, user-owned regular file");
    }
    let bytes = fs::read(path).context("failed to read curtain ownership record")?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        bail!("curtain ownership record exceeds the size limit");
    }
    let record: OwnerRecord =
        serde_json::from_slice(&bytes).context("failed to decode curtain ownership record")?;
    let runtime_dir = path
        .parent()
        .context("ownership record has no parent directory")?;
    record.validate(session, runtime_dir)?;
    Ok(Some(record))
}

pub(crate) fn publish(path: &Path, record: &OwnerRecord) -> Result<()> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock predates Unix epoch")?
        .as_nanos();
    let staging = path.with_extension(format!("{}.{stamp}.staging", std::process::id()));
    let bytes = serde_json::to_vec(record).context("failed to encode curtain ownership")?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        bail!("curtain ownership record exceeds the size limit");
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staging)
        .with_context(|| format!("failed to create {}", staging.display()))?;
    let result = (|| -> Result<()> {
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&staging, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staging);
    }
    result.context("failed to publish curtain ownership record")
}

pub(crate) fn remove(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("failed to remove curtain ownership record"),
    }
}

pub(crate) fn process_start_ticks(pid: u32) -> Result<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .with_context(|| format!("failed to inspect curtain process {pid}"))?;
    parse_process_stat(&stat).map(|(_, ticks)| ticks)
}

pub(crate) fn process_matches(pid: u32, start_ticks: u64) -> Result<bool> {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => {
            let (state, actual) = parse_process_stat(&stat)?;
            Ok(actual == start_ticks && state != 'Z' && state != 'X')
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("failed to inspect adopted curtain process"),
    }
}

fn parse_process_stat(stat: &str) -> Result<(char, u64)> {
    let (_, fields) = stat.rsplit_once(") ").context("malformed process stat")?;
    let mut fields = fields.split_whitespace();
    let state = fields
        .next()
        .and_then(|field| field.chars().next())
        .context("process stat has no state")?;
    let ticks = fields
        .nth(18)
        .context("process stat has no start time")?
        .parse()
        .context("invalid process start time")?;
    Ok((state, ticks))
}

#[cfg(test)]
mod tests;
