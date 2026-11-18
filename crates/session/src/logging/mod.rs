pub mod rotation;

use nix::fcntl::{FcntlArg, OFlag, fcntl};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, PipeReader, Read, Write},
    os::{
        fd::{AsFd, OwnedFd},
        unix::{
            fs::{OpenOptionsExt, PermissionsExt},
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use veila_common::AppConfig;

pub const LOG_SUBCOMMAND: &str = "__lock-log";
const LOG_PATH_ENV: &str = "VEILA_LOCK_LOG_PATH";

pub fn open_private_log(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NONBLOCK)
        .open(path)?;
    if file.metadata()?.is_file() {
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(file)
}

pub(crate) fn prepare(config: &AppConfig) -> io::Result<Option<(File, LogReader)>> {
    if !config.lock.log_to_file {
        return Ok(None);
    }
    let path = if let (Ok(rest), Some(home)) = (
        config.lock.log_file_path.strip_prefix("~"),
        std::env::var_os("HOME"),
    ) {
        PathBuf::from(home).join(rest)
    } else {
        config.lock.log_file_path.clone()
    };
    let (reader, writer) = io::pipe()?;
    let flags = OFlag::from_bits_truncate(fcntl(&writer, FcntlArg::F_GETFL)?);
    fcntl(&writer, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK))?;
    let _ = fcntl(&writer, FcntlArg::F_SETPIPE_SZ(256 * 1024));
    let writer = File::from(OwnedFd::from(writer));
    let mut marker = &writer;
    marker.write_all(b"starting per-lock supervisor\n")?;
    Ok(Some((writer, LogReader { reader, path })))
}

pub(crate) struct LogReader {
    reader: PipeReader,
    path: PathBuf,
}
impl LogReader {
    pub fn spawn(self, writer: File) -> io::Result<LogChild> {
        let stderr = io::stderr().as_fd().try_clone_to_owned()?;
        // Follow pipe EOF so an orphaned curtain can still log and exit independently.
        let child = Command::new("/proc/self/exe")
            .arg(LOG_SUBCOMMAND)
            .env(LOG_PATH_ENV, self.path)
            .stdin(self.reader)
            .stdout(Stdio::null())
            .process_group(0)
            .spawn()?;
        if let Err(error) = nix::unistd::dup2_stderr(writer) {
            tracing::warn!(%error, "could not redirect supervisor logging");
        }
        Ok(LogChild { child, stderr })
    }
}

pub(crate) struct LogChild {
    child: Child,
    stderr: OwnedFd,
}

impl LogChild {
    pub fn reap(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(status)) => {
                let _ = nix::unistd::dup2_stderr(&self.stderr);
                tracing::warn!(%status, "lock log writer exited; using stderr");
                true
            }
            Ok(None) => false,
            Err(error) => {
                tracing::warn!(%error, "could not check lock log writer");
                false
            }
        }
    }
}

#[doc(hidden)]
pub fn run() -> io::Result<()> {
    let path = std::env::var_os(LOG_PATH_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing lock log path"))?;
    let mut target = match open_target(&path) {
        Ok(log) => Some(log),
        Err(error) => {
            tracing::warn!(%error, "optional log unavailable; using stderr");
            None
        }
    };
    let mut input = io::stdin().lock();
    let mut entry = Vec::new();
    let mut warned = false;
    loop {
        entry.clear();
        if (&mut input).take(64 * 1024).read_until(b'\n', &mut entry)? == 0 {
            return Ok(());
        }
        if let Some(log) = &mut target {
            match log.write_all(&entry) {
                Ok(()) => {
                    warned = false;
                    continue;
                }
                Err(error) if !warned => {
                    warned = true;
                    tracing::warn!(%error, "could not append to lock log; using stderr");
                }
                Err(_) => {}
            }
        }
        let _ = io::stderr().write_all(&entry);
    }
}

fn open_target(path: &Path) -> io::Result<rotation::RotatingLog> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    rotation::RotatingLog::new(open_private_log(path)?, path)
}
