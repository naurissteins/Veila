use std::{
    fmt,
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use anyhow::Context;
use time::{OffsetDateTime, UtcOffset};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::FormatTime;
use veila_common::AppConfig;

struct ShortLocalTime;

impl FormatTime for ShortLocalTime {
    fn format_time(&self, writer: &mut tracing_subscriber::fmt::format::Writer<'_>) -> fmt::Result {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = veila_daemon::DaemonOptions::parse_args(std::env::args())?;
    let _log_guard = init_tracing(&options)?;

    if !options.help
        && !options.stop
        && options.print_theme.is_none()
        && options.set_theme.is_none()
        && !options.unset_theme
        && !options.list_themes
        && !options.status
        && !options.health
        && !options.version
        && !options.reload_config
    {
        tracing::info!(
            component = veila_daemon::component_name(),
            "starting daemon"
        );
    }

    veila_daemon::run(options).await
}

fn init_tracing(options: &veila_daemon::DaemonOptions) -> anyhow::Result<Option<WorkerGuard>> {
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into());

    if let Some(path) = resolved_log_file_path(options)? {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create daemon log directory {}", parent.display())
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("failed to open daemon log file {}", path.display()))?;
        let (writer, guard) = tracing_appender::non_blocking(file);

        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_timer(ShortLocalTime)
            .with_writer(writer)
            .init();
        return Ok(Some(guard));
    }

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_timer(ShortLocalTime)
        .init();
    Ok(None)
}

fn resolved_log_file_path(
    options: &veila_daemon::DaemonOptions,
) -> anyhow::Result<Option<PathBuf>> {
    if !starts_daemon(options) {
        return Ok(None);
    }

    if let Some(path) = options.log_file_path.as_deref() {
        return Ok(Some(normalize_log_file_path(path)));
    }

    let loaded = AppConfig::load(options.config_path.as_deref())
        .context("failed to load daemon config for log setup")?;
    if !loaded.config.lock.log_to_file {
        return Ok(None);
    }

    Ok(Some(normalize_log_file_path(
        loaded.config.lock.log_file_path.as_path(),
    )))
}

fn starts_daemon(options: &veila_daemon::DaemonOptions) -> bool {
    !options.help
        && !options.stop
        && options.print_theme.is_none()
        && options.set_theme.is_none()
        && !options.unset_theme
        && !options.list_themes
        && !options.status
        && !options.health
        && !options.version
        && !options.reload_config
}

fn normalize_log_file_path(path: &Path) -> PathBuf {
    let Some(raw) = path.to_str() else {
        return path.to_path_buf();
    };

    if raw == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf());
    }

    if let Some(rest) = raw.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }

    path.to_path_buf()
}
