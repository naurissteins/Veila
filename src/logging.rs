use std::{
    fmt,
    fs::{self, File, OpenOptions},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

use anyhow::{Context, Result};
use time::{OffsetDateTime, UtcOffset};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt::format::Writer, fmt::time::FormatTime};

struct ShortLocalTime;

impl FormatTime for ShortLocalTime {
    fn format_time(&self, writer: &mut Writer<'_>) -> fmt::Result {
        let now = OffsetDateTime::now_utc()
            .to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
        write!(
            writer,
            "{:02}:{:02}:{:02}",
            now.hour(),
            now.minute(),
            now.second()
        )
    }
}

pub(crate) fn init_stderr() {
    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .with_timer(ShortLocalTime)
        .init();
}

pub(crate) fn init_daemon(log_file: Option<&Path>) -> Result<Option<WorkerGuard>> {
    let Some(path) = log_file else {
        init_stderr();
        return Ok(None);
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create daemon log directory {}", parent.display())
        })?;
    }

    let file = open_private_log(path)?;
    let (writer, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .with_timer(ShortLocalTime)
        .with_writer(writer)
        .init();
    Ok(Some(guard))
}

fn open_private_log(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("failed to open daemon log file {}", path.display()))?;
    if file.metadata()?.is_file() {
        // Existing logs may have been created with the user's default umask.
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to secure daemon log file {}", path.display()))?;
    }
    Ok(file)
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
    };

    use super::open_private_log;

    #[test]
    fn opening_an_existing_log_removes_public_read_access() {
        let root = std::env::temp_dir().join(format!("veila-private-log-{}", std::process::id()));
        fs::create_dir_all(&root).expect("log directory");
        let path = root.join("veila.log");
        fs::write(&path, b"old log\n").expect("old log");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("old mode");

        let file = open_private_log(&path).expect("private log");

        assert_eq!(file.metadata().expect("log metadata").mode() & 0o777, 0o600);
        assert_eq!(fs::read(&path).expect("log contents"), b"old log\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn new_log_file_is_owner_only() {
        let root = std::env::temp_dir().join(format!("veila-new-log-{}", std::process::id()));
        fs::create_dir_all(&root).expect("log directory");
        let path = root.join("veila.log");

        let file = open_private_log(&path).expect("private log");

        assert_eq!(file.metadata().expect("log metadata").mode() & 0o777, 0o600);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
