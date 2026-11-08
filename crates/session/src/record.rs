use anyhow::{Context, Result, bail};
use nix::unistd::Uid;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    hash::{DefaultHasher, Hash, Hasher},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Record {
    pub supervisor_pid: u32,
    pub supervisor_start_ticks: u64,
    pub curtain_pid: Option<u32>,
    pub curtain_start_ticks: Option<u64>,
    pub state: State,
    #[serde(default)]
    pub ready: Option<bool>,
    pub config: Option<PathBuf>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum State {
    Starting,
    Locked,
    Unlocked,
    Refused,
    Abnormal,
}

pub(crate) struct LockRecord {
    pub file: File,
    pub value: Record,
    pub recovered: bool,
}

impl LockRecord {
    pub(super) fn open_at(runtime: PathBuf, display: PathBuf) -> Result<File> {
        open_record(runtime, display, "lock")
    }
    pub(super) fn at(
        runtime: PathBuf,
        display: PathBuf,
        config: Option<PathBuf>,
    ) -> Result<Option<Self>> {
        let mut file = Self::open_at(runtime, display)?;
        if !acquire_exclusive(&file)? {
            return Ok(None);
        }
        let mut previous = Vec::new();
        (&mut file).take(4097).read_to_end(&mut previous)?;
        let recovered = !previous.is_empty()
            && serde_json::from_slice::<Record>(&previous).map_or(true, |r| {
                !matches!(r.state, State::Unlocked | State::Refused)
            });
        let value = Record {
            supervisor_pid: std::process::id(),
            supervisor_start_ticks: start_ticks(std::process::id())?,
            curtain_pid: None,
            curtain_start_ticks: None,
            state: State::Starting,
            ready: Some(false),
            config,
        };
        let mut record = Self {
            file,
            value,
            recovered,
        };
        record.save()?;
        Ok(Some(record))
    }
    pub(crate) fn active() -> Result<Option<State>> {
        let runtime = PathBuf::from(
            std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is required")?,
        );
        let display = PathBuf::from(
            std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into()),
        );
        Self::active_at(runtime, display)
    }
    fn active_at(runtime: PathBuf, display: PathBuf) -> Result<Option<State>> {
        let Some(bytes) = held_bytes(Self::open_at(runtime, display)?)? else {
            return Ok(None);
        };
        // A partial record never proves compositor confirmation.
        Ok(Some(
            serde_json::from_slice::<Record>(&bytes).map_or(State::Starting, |r| r.state),
        ))
    }
    pub fn save(&mut self) -> Result<()> {
        let bytes = serde_json::to_vec(&self.value)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&bytes)?;
        self.file.set_len(bytes.len() as u64)?;
        Ok(())
    }
}
use std::os::unix::fs::DirBuilderExt;
pub(crate) fn start_ticks(pid: u32) -> Result<u64> {
    let text = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let tail = text.rsplit_once(')').context("invalid process identity")?.1;
    if matches!(tail.split_whitespace().next(), Some("Z" | "X" | "x")) {
        bail!("process has exited");
    }
    tail.split_whitespace()
        .nth(19)
        .context("invalid process identity")?
        .parse()
        .context("invalid process start time")
}

#[cfg(test)]
mod tests;

pub(super) fn open_record(runtime: PathBuf, display: PathBuf, kind: &str) -> Result<File> {
    let meta = fs::symlink_metadata(&runtime)?;
    if !meta.is_dir() || meta.uid() != Uid::effective().as_raw() || meta.mode() & 0o077 != 0 {
        bail!("runtime directory must be private and owned by this user");
    }
    let directory = runtime.join("veila");
    match fs::DirBuilder::new().mode(0o700).create(&directory) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    let meta = fs::symlink_metadata(&directory)?;
    if !meta.is_dir() || meta.uid() != Uid::effective().as_raw() || meta.mode() & 0o077 != 0 {
        bail!("invalid Veila runtime directory");
    }
    let display = if display.is_absolute() {
        display
    } else {
        runtime.join(display)
    };
    let display = fs::canonicalize(&display).unwrap_or(display);
    let mut hash = DefaultHasher::new();
    display.hash(&mut hash);
    let path = directory.join(format!("{kind}-{:016x}", hash.finish()));
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.uid() != Uid::effective().as_raw()
        || meta.nlink() != 1
        || meta.mode() & 0o077 != 0
    {
        bail!("invalid lock record file");
    }
    Ok(file)
}

pub(super) fn environment() -> Result<(PathBuf, PathBuf)> {
    Ok((
        PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is required")?),
        PathBuf::from(std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into())),
    ))
}

pub(super) fn acquire_exclusive(file: &File) -> Result<bool> {
    for _ in 0..32 {
        match file.try_lock() {
            Ok(()) => return Ok(true),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(error) => return Err(error.into()),
        }
        // A status reader must never be mistaken for an active lock supervisor.
        match file.try_lock_shared() {
            Ok(()) => {
                file.unlock()?;
                std::thread::yield_now();
            }
            Err(std::fs::TryLockError::WouldBlock) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
    bail!("lock record is busy with status readers; retry locking")
}

pub(super) fn held_bytes(mut file: File) -> Result<Option<Vec<u8>>> {
    match file.try_lock_shared() {
        Ok(()) => return Ok(None),
        Err(std::fs::TryLockError::WouldBlock) => {}
        Err(error) => return Err(error.into()),
    }
    let mut bytes = Vec::new();
    (&mut file).take(4097).read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}
