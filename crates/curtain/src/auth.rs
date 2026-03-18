use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::mpsc::Sender,
    thread,
    time::Instant,
};

use kwylock_common::ipc::{ClientMessage, DaemonMessage, decode_message, encode_message};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthEvent {
    Rejected { retry_after_ms: Option<u64> },
    Busy,
}

pub(crate) fn submit_password(socket_path: PathBuf, secret: String, sender: Sender<AuthEvent>) {
    thread::spawn(move || {
        if let Err(error) = run_attempt(socket_path, secret, sender) {
            tracing::warn!("failed to submit password attempt: {error:#}");
        }
    });
}

fn run_attempt(
    socket_path: PathBuf,
    secret: String,
    sender: Sender<AuthEvent>,
) -> anyhow::Result<()> {
    let started_at = Instant::now();
    let mut stream = UnixStream::connect(&socket_path)?;
    let mut payload = encode_message(&ClientMessage::SubmitPassword { secret })?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    tracing::debug!(
        elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        "submitted authentication request to daemon"
    );

    let mut line = String::new();
    let read = BufReader::new(stream).read_line(&mut line)?;
    if read == 0 {
        tracing::info!(
            elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            "authentication socket closed without response; assuming success path"
        );
        return Ok(());
    }

    match decode_message::<DaemonMessage>(line.trim_end())? {
        DaemonMessage::AuthenticationRejected { retry_after_ms } => {
            tracing::info!(
                elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                "daemon rejected authentication request"
            );
            let _ = sender.send(AuthEvent::Rejected { retry_after_ms });
        }
        DaemonMessage::AuthenticationBusy => {
            tracing::debug!(
                elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                "daemon reported authentication request is busy"
            );
            let _ = sender.send(AuthEvent::Busy);
        }
    }

    Ok(())
}
