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
    pub fn acquire(config: Option<PathBuf>) -> Result<Option<Self>> {
        let runtime = PathBuf::from(
            std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is required")?,
        );
        let display = PathBuf::from(
            std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into()),
        );
        Self::at(runtime, display, config)
    }
    fn at(runtime: PathBuf, display: PathBuf, config: Option<PathBuf>) -> Result<Option<Self>> {
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
        let path = directory.join(format!("lock-{:016x}", hash.finish()));
        let mut file = OpenOptions::new()
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
        match file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
            Err(e) => return Err(e.into()),
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
    text.rsplit_once(')')
        .and_then(|(_, tail)| tail.split_whitespace().nth(19))
        .context("invalid process identity")?
        .parse()
        .context("invalid process start time")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "veila-record-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("root");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("mode");
        root
    }
    #[test]
    fn inherited_file_keeps_record_locked_after_supervisor_drop() {
        let root = root();
        let display = PathBuf::from("wayland-test");
        let mut record = LockRecord::at(root.clone(), display.clone(), None)
            .expect("record")
            .expect("lock");
        let inherited = record.file.try_clone().expect("inherit");
        record.value.state = State::Locked;
        record.save().expect("save");
        drop(record);
        assert!(
            LockRecord::at(root.clone(), display.clone(), None)
                .expect("check")
                .is_none()
        );
        drop(inherited);
        let next = LockRecord::at(root.clone(), display, None)
            .expect("next")
            .expect("lock");
        assert!(next.recovered);
        drop(next);
        fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn unlocked_record_reuses_inode_and_displays_are_independent() {
        let root = root();
        let a = PathBuf::from("a");
        let mut record = LockRecord::at(root.clone(), a.clone(), None)
            .expect("record")
            .expect("lock");
        let inode = record.file.metadata().expect("meta").ino();
        let other = LockRecord::at(root.clone(), "b".into(), None)
            .expect("other")
            .expect("lock");
        drop(other);
        record.value.state = State::Unlocked;
        record.save().expect("save");
        drop(record);
        let next = LockRecord::at(root.clone(), a, None)
            .expect("next")
            .expect("lock");
        assert!(!next.recovered);
        assert_eq!(inode, next.file.metadata().expect("meta").ino());
        drop(next);
        fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn rejects_insecure_or_symlinked_directories() {
        let root = root();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).expect("mode");
        assert!(LockRecord::at(root.clone(), "a".into(), None).is_err());
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("mode");
        std::os::unix::fs::symlink("/tmp", root.join("veila")).expect("symlink");
        assert!(LockRecord::at(root.clone(), "a".into(), None).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn torn_non_utf8_record_retains_recovery_backoff_instead_of_preventing_lock() {
        let root = root();
        let display = PathBuf::from("wayland-test");
        let mut record = LockRecord::at(root.clone(), display.clone(), None)
            .expect("record")
            .expect("lock");
        record.file.seek(SeekFrom::Start(0)).expect("seek");
        record
            .file
            .write_all(b"{\"config\":\"\xff")
            .expect("torn record");
        record.file.set_len(12).expect("truncate");
        drop(record);
        let recovered = LockRecord::at(root.clone(), display, None)
            .expect("read malformed bytes")
            .expect("lock");
        assert!(recovered.recovered);
        drop(recovered);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
