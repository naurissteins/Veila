mod follow;
mod journal;

use crate::{
    logging::normalize_log_file_path,
    options::{LogOptions, LogSource, LogTarget},
};
use anyhow::{Context, Result, bail};
use std::{
    collections::VecDeque,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
use veila_common::AppConfig;

pub async fn print_logs(config_path: Option<&Path>, options: &LogOptions) -> Result<()> {
    let journal = options.source != LogSource::File;
    let file = options.source != LogSource::Journal
        && options.target != LogTarget::Idle
        && options.since.is_none();
    let path = if file {
        match log_path(config_path) {
            Ok(path) => path,
            Err(error) if journal => {
                println!("lock_file.unavailable={error}");
                None
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    if options.follow {
        return follow::run(
            path.as_deref(),
            journal.then(|| self::journal::command(options)),
            options,
        )
        .await;
    }
    if let Some(path) = path {
        println!("--- lock log: {} ---", path.display());
        match print_recent_file_lines(&path, options.lines.unwrap_or(200), options.target) {
            Ok(()) => {}
            Err(error) if journal => println!("lock_file.unavailable={error}"),
            Err(error) => return Err(error),
        }
    }
    if journal {
        println!("--- idle watcher journal ---");
        std::io::stdout().flush()?;
        let status = self::journal::command(options)
            .status()
            .context("failed to run journalctl")?;
        if !status.success() {
            bail!("journalctl exited with {status}");
        }
    }
    Ok(())
}

fn log_path(config_path: Option<&Path>) -> Result<Option<PathBuf>> {
    let loaded = AppConfig::load(config_path).context("failed to load config for file log path")?;
    if !loaded.config.lock.log_to_file {
        println!("lock_file.disabled=true; enable [lock].log_to_file for future lock diagnostics");
        return Ok(None);
    }
    Ok(Some(normalize_log_file_path(
        &loaded.config.lock.log_file_path,
    )))
}

fn matches_target(line: &str, target: LogTarget) -> bool {
    match target {
        LogTarget::All => true,
        LogTarget::Curtain => line.contains("veila_curtain"),
        LogTarget::Ui => line.contains("veila_ui"),
        LogTarget::Idle => false,
    }
}

fn print_recent_file_lines(path: &Path, lines: u32, target: LogTarget) -> Result<()> {
    if lines == 0 {
        return Ok(());
    }
    let file =
        File::open(path).with_context(|| format!("failed to open log file {}", path.display()))?;
    let mut recent = VecDeque::new();
    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("failed to read log file {}", path.display()))?;
        if !matches_target(&line, target) {
            continue;
        }
        if recent.len() == lines as usize {
            recent.pop_front();
        }
        recent.push_back(line);
    }
    for line in recent {
        println!("{line}");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
