pub mod rotation;

use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};
use veila_common::AppConfig;

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

pub(crate) fn open(config: &AppConfig) -> io::Result<Option<SessionLog>> {
    if !config.lock.log_to_file {
        return Ok(None);
    }
    let path = if let (Ok(rest), Some(home)) = (
        config.lock.log_file_path.strip_prefix("~"),
        std::env::var_os("HOME"),
    ) {
        std::path::PathBuf::from(home).join(rest)
    } else {
        config.lock.log_file_path.clone()
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = open_private_log(&path)?;
    // rotate before sharing this inode; curtain writes remain ordinary stderr writes
    use std::io::Write;
    let mut writer = rotation::RotatingLog::new(file.try_clone()?, &path)?;
    writer.write_all(b"starting per-lock supervisor\n")?;
    Ok(Some(SessionLog {
        file,
        path,
        checked: std::time::Instant::now(),
    }))
}

pub(crate) struct SessionLog {
    pub file: File,
    path: std::path::PathBuf,
    checked: std::time::Instant,
}
impl SessionLog {
    pub fn maintain(&mut self) {
        if self.checked.elapsed() < std::time::Duration::from_secs(15) {
            return;
        }
        self.checked = std::time::Instant::now();
        let result = (|| -> io::Result<()> {
            use std::io::Write;
            let mut writer = rotation::RotatingLog::new(self.file.try_clone()?, &self.path)?;
            writer.write_all(b"")
        })();
        if let Err(error) = result {
            tracing::warn!(%error,"could not maintain lock log");
        }
    }
}
