use std::{
    io::{BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use calloop::channel::Sender;

use super::read_bounded_line;
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::Uid;
use veila_common::{
    PowerAction, Secret, elapsed_ms,
    ipc::{ClientMessage, DaemonMessage, decode_message, encode_message, encode_secret_message},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthEvent {
    Challenge {
        attempt_id: u64,
        sequence: u32,
        echo: bool,
        text: String,
    },
    Notice {
        attempt_id: u64,
        text: String,
    },
    Accepted {
        attempt_id: u64,
    },
    Rejected {
        attempt_id: u64,
        retry_after_ms: Option<u64>,
        failed_attempts: Option<u8>,
        message: Option<String>,
    },
    Busy {
        attempt_id: u64,
    },
    /// The attempt produced no verdict; the daemon was unreachable, silent, or too slow.
    Failed {
        attempt_id: u64,
    },
}

pub(crate) enum ChallengeReply {
    Response { sequence: u32, secret: Secret },
    Cancel { sequence: u32 },
}

const AUTH_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const AUTH_RESPONSE_TIMEOUT: Duration = Duration::from_secs(125);

pub(crate) fn submit_password(
    socket_path: PathBuf,
    attempt_id: u64,
    secret: Secret,
    sender: Sender<AuthEvent>,
) -> mpsc::Sender<ChallengeReply> {
    let (reply_sender, reply_receiver) = mpsc::channel();
    thread::spawn(move || {
        if let Err(error) = run_attempt(socket_path, attempt_id, secret, &sender, &reply_receiver) {
            tracing::warn!(attempt_id, "failed to submit password attempt: {error:#}");
            let _ = sender.send(AuthEvent::Failed { attempt_id });
        }
    });
    reply_sender
}

pub(crate) fn request_power_action(socket_path: PathBuf, action: PowerAction) {
    thread::spawn(move || {
        if let Err(error) = run_power_action_request(socket_path, action) {
            tracing::warn!(?action, "failed to request power action: {error:#}");
        }
    });
}

fn run_attempt(
    socket_path: PathBuf,
    attempt_id: u64,
    secret: Secret,
    sender: &Sender<AuthEvent>,
    reply_receiver: &mpsc::Receiver<ChallengeReply>,
) -> anyhow::Result<()> {
    let started_at = Instant::now();
    let mut stream = UnixStream::connect(&socket_path)?;
    verify_socket_peer(&stream).context("auth socket peer rejected")?;
    stream
        .set_write_timeout(Some(AUTH_WRITE_TIMEOUT))
        .context("failed to set auth write timeout")?;
    stream
        .set_read_timeout(Some(AUTH_RESPONSE_TIMEOUT))
        .context("failed to set auth response timeout")?;
    let mut payload = encode_secret_message(&ClientMessage::SubmitPassword { attempt_id, secret })?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    stream.flush()?;
    drop(payload);
    tracing::debug!(
        attempt_id,
        elapsed_ms = elapsed_ms(started_at),
        "submitted authentication request to daemon"
    );

    let mut reader = BufReader::new(stream);
    let mut last_sequence = 0_u32;
    let mut notices = 0_u32;
    loop {
        let Some(line) = read_bounded_line(&mut reader, "auth response")? else {
            bail!("daemon closed the auth socket without sending a verdict");
        };
        match decode_message::<DaemonMessage>(&line)? {
            DaemonMessage::AuthenticationChallenge {
                attempt_id: id,
                sequence,
                echo,
                text,
            } if id == attempt_id => {
                if sequence != last_sequence + 1 || sequence > 16 || !valid_pam_text(&text) {
                    bail!("daemon sent invalid PAM challenge");
                }
                last_sequence = sequence;
                let _ = sender.send(AuthEvent::Challenge {
                    attempt_id,
                    sequence,
                    echo,
                    text,
                });
                let reply = reply_receiver
                    .recv_timeout(Duration::from_secs(60))
                    .context("curtain did not answer PAM challenge")?;
                let message = match reply {
                    ChallengeReply::Response {
                        sequence: answer,
                        secret,
                    } if answer == sequence => ClientMessage::AuthenticationResponse {
                        attempt_id,
                        sequence,
                        secret,
                    },
                    ChallengeReply::Cancel { sequence: answer } if answer == sequence => {
                        ClientMessage::CancelAuthentication {
                            attempt_id,
                            sequence,
                        }
                    }
                    _ => bail!("curtain sent an out-of-order PAM response"),
                };
                let mut payload = encode_secret_message(&message)?;
                payload.push(b'\n');
                reader.get_mut().write_all(&payload)?;
                reader.get_mut().flush()?;
            }
            DaemonMessage::AuthenticationNotice {
                attempt_id: id,
                text,
            } if id == attempt_id => {
                notices += 1;
                if notices > 32 || !valid_pam_text(&text) {
                    bail!("daemon sent invalid PAM notice");
                }
                let _ = sender.send(AuthEvent::Notice { attempt_id, text });
            }
            DaemonMessage::AuthenticationAccepted { attempt_id: id } if id == attempt_id => {
                tracing::info!(
                    elapsed_ms = elapsed_ms(started_at),
                    attempt_id,
                    "daemon accepted authentication request"
                );
                let _ = sender.send(AuthEvent::Accepted { attempt_id });
                break;
            }
            DaemonMessage::AuthenticationRejected {
                attempt_id: id,
                retry_after_ms,
                failed_attempts,
                message,
            } if id == attempt_id && message.as_deref().is_none_or(valid_pam_text) => {
                tracing::info!(
                    elapsed_ms = elapsed_ms(started_at),
                    attempt_id,
                    "daemon rejected authentication request"
                );
                let _ = sender.send(AuthEvent::Rejected {
                    attempt_id,
                    retry_after_ms,
                    failed_attempts,
                    message,
                });
                break;
            }
            DaemonMessage::AuthenticationBusy { attempt_id: id } if id == attempt_id => {
                tracing::debug!(
                    elapsed_ms = elapsed_ms(started_at),
                    attempt_id,
                    "daemon reported authentication request is busy"
                );
                let _ = sender.send(AuthEvent::Busy { attempt_id });
                break;
            }
            DaemonMessage::Error { reason } => {
                tracing::warn!(
                    attempt_id,
                    "daemon rejected authentication request: {reason}"
                );
                let _ = sender.send(AuthEvent::Failed { attempt_id });
                break;
            }
            _ => bail!("unexpected authentication response"),
        }
    }

    Ok(())
}

fn valid_pam_text(text: &str) -> bool {
    text.chars().count() <= 160 && !text.chars().any(char::is_control)
}

fn run_power_action_request(socket_path: PathBuf, action: PowerAction) -> anyhow::Result<()> {
    let mut stream = UnixStream::connect(&socket_path)?;
    verify_socket_peer(&stream).context("auth socket peer rejected")?;
    let mut payload = encode_message(&ClientMessage::RequestPowerAction { action })?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub(super) fn verify_socket_peer(stream: &UnixStream) -> Result<()> {
    let expected_uid = Uid::effective().as_raw();
    let peer = getsockopt(stream, PeerCredentials).context("failed to read peer credentials")?;
    if peer.uid() != expected_uid {
        bail!(
            "peer uid {} does not match curtain uid {}",
            peer.uid(),
            expected_uid
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
