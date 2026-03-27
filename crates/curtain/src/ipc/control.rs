use std::{
    io::{BufRead, BufReader},
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::mpsc::Sender,
    thread,
};

use anyhow::{Context, Result};
use veila_common::NowPlayingSnapshot;
use veila_common::ipc::{CurtainControlMessage, decode_message};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlEvent {
    Unlock {
        attempt_id: Option<u64>,
    },
    Reload,
    UpdateNowPlaying {
        snapshot: Option<NowPlayingSnapshot>,
    },
}

pub(crate) fn spawn_listener(socket_path: PathBuf, sender: Sender<ControlEvent>) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).with_context(|| {
            format!(
                "failed to remove stale control socket {}",
                socket_path.display()
            )
        })?;
    }

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind control socket {}", socket_path.display()))?;

    thread::spawn(move || {
        if let Err(error) = run_listener(listener, sender) {
            tracing::warn!("curtain control listener exited with error: {error:#}");
        }
    });

    Ok(())
}

fn run_listener(listener: UnixListener, sender: Sender<ControlEvent>) -> Result<()> {
    loop {
        let (stream, _) = listener
            .accept()
            .context("failed to accept control connection")?;
        let mut line = String::new();
        let read = BufReader::new(stream)
            .read_line(&mut line)
            .context("failed to read control message")?;

        if read == 0 {
            continue;
        }

        match decode_message::<CurtainControlMessage>(line.trim_end())
            .context("invalid curtain control message")?
        {
            CurtainControlMessage::Unlock { attempt_id } => {
                let _ = sender.send(ControlEvent::Unlock { attempt_id });
                return Ok(());
            }
            CurtainControlMessage::ReloadConfig => {
                let _ = sender.send(ControlEvent::Reload);
            }
            CurtainControlMessage::UpdateNowPlaying { snapshot } => {
                let _ = sender.send(ControlEvent::UpdateNowPlaying { snapshot });
            }
        }
    }
}
