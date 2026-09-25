use std::{fmt, fs::OpenOptions, path::Path};

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

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open daemon log file {}", path.display()))?;
    let (writer, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .with_timer(ShortLocalTime)
        .with_writer(writer)
        .init();
    Ok(Some(guard))
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}
