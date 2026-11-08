use crate::options::LogOptions;
use anyhow::{Context, Result, bail};
use std::{
    path::Path,
    process::{Command, Stdio},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStdout},
};

struct Stream {
    child: Child,
    lines: Lines<BufReader<ChildStdout>>,
    file: bool,
}
impl Stream {
    fn spawn(command: Command, file: bool) -> Result<Self> {
        let mut command = tokio::process::Command::from(command);
        let mut child = command
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("failed to start log follower")?;
        let stdout = child
            .stdout
            .take()
            .context("log follower stdout unavailable")?;
        Ok(Self {
            child,
            lines: BufReader::new(stdout).lines(),
            file,
        })
    }
}

pub(super) async fn run(
    path: Option<&Path>,
    journal: Option<Command>,
    options: &LogOptions,
) -> Result<()> {
    let mut file = if let Some(path) = path {
        println!("--- following lock log: {} ---", path.display());
        let mut command = Command::new("tail");
        // Follow the name across log rotation and locks that create the file later.
        command
            .args(["-n", &options.lines.unwrap_or(50).to_string(), "-F", "--"])
            .arg(path);
        Some(Stream::spawn(command, true)?)
    } else {
        None
    };
    let mut journal = journal
        .map(|command| Stream::spawn(command, false))
        .transpose()?;
    if file.is_none() && journal.is_none() {
        return Ok(());
    }
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let result = tokio::select! {
        result = pump(&mut file, options) => result,
        result = pump(&mut journal, options) => result,
        result = tokio::signal::ctrl_c() => result.context("failed to receive interrupt"),
        _ = terminate.recv() => Ok(()),
    };
    // Reap both followers on errors and signals so neither keeps writing after this CLI exits.
    for stream in [file, journal].into_iter().flatten() {
        let mut child = stream.child;
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}

async fn pump(stream: &mut Option<Stream>, options: &LogOptions) -> Result<()> {
    let Some(stream) = stream else {
        return std::future::pending().await;
    };
    while let Some(line) = stream
        .lines
        .next_line()
        .await
        .context("failed to read log follower")?
    {
        if !stream.file || super::matches_target(&line, options.target) {
            println!("{line}");
        }
    }
    let status = stream.child.wait().await?;
    if !status.success() {
        bail!("log follower exited with {status}");
    }
    Ok(())
}
