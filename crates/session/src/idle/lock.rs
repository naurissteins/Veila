use anyhow::{Context, Result};
use std::{
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::Path,
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    sync::mpsc,
};
use veila_common::{ipc::CurtainStartupMessage, session::fd};

#[derive(Debug)]
pub(super) enum Event {
    Confirmed,
    Exited,
}

pub(super) fn spawn(path: Option<&Path>, events: mpsc::Sender<Event>) -> Result<()> {
    let (notify, inherited) = std::os::unix::net::UnixStream::pair()?;
    notify.set_nonblocking(true)?;
    let notify = tokio::net::UnixStream::from_std(notify)?;
    let mut command = tokio::process::Command::new("/proc/self/exe");
    command
        .arg("lock")
        .arg(format!("--notify-fd={}", inherited.as_raw_fd()))
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    command.as_std_mut().process_group(0);
    if let Some(path) = path {
        command.arg("--config").arg(path);
    }
    fd::inheritable(&inherited, true)?;
    let result = command.spawn();
    let flags = fd::inheritable(&inherited, false);
    drop(inherited);
    let mut child = result.context("failed to spawn lock supervisor")?;
    // Reaping is independent of confirmation and never blocks idle or sleep signals.
    tokio::spawn(async move {
        if let Err(error) = flags {
            tracing::warn!(%error, "failed to restore notification flags");
        }
        let mut line = Vec::new();
        let result = BufReader::new(notify.take(4096))
            .read_until(b'\n', &mut line)
            .await;
        if result.is_ok()
            && matches!(
                serde_json::from_slice::<CurtainStartupMessage>(&line),
                Ok(CurtainStartupMessage::SessionLocked)
            )
        {
            let _ = events.send(Event::Confirmed).await;
        }
        match child.wait().await {
            Ok(status) => tracing::info!(%status, "idle watcher reaped lock supervisor"),
            Err(error) => tracing::error!(%error, "failed to reap lock supervisor"),
        }
        let _ = events.send(Event::Exited).await;
    });
    Ok(())
}
