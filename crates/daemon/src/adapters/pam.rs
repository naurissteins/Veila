use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    net::UnixStream,
    process::Command,
    time::timeout,
};
use veila_auth::protocol::{
    self, FrameBuffer, HelperMessage, HelperRequest, INITIAL_DEADLINE, INTERACTIVE_DEADLINE,
    PROMPT_DEADLINE, WRITE_DEADLINE,
};
use veila_common::{
    Secret,
    ipc::{ClientMessage, DaemonMessage},
};

use super::ipc;

pub struct PamReply {
    pub accepted: bool,
    pub message: Option<String>,
}

pub async fn authenticate(
    username: &str,
    secret: Secret,
    attempt_id: u64,
    stream: &mut UnixStream,
) -> Result<PamReply> {
    let mut child = Command::from(veila_auth::helper_command())
        .kill_on_drop(true)
        .spawn()
        .context("failed to start PAM helper")?;
    run_with_cleanup(
        &mut child,
        username,
        secret,
        attempt_id,
        stream,
        INITIAL_DEADLINE,
        INTERACTIVE_DEADLINE,
    )
    .await
}

async fn run_with_cleanup(
    child: &mut tokio::process::Child,
    username: &str,
    secret: Secret,
    attempt_id: u64,
    stream: &mut UnixStream,
    initial_deadline: Duration,
    interactive_deadline: Duration,
) -> Result<PamReply> {
    let result = exchange(
        child,
        username,
        secret,
        attempt_id,
        stream,
        initial_deadline,
        interactive_deadline,
    )
    .await;
    if result.is_err() {
        let _ = child.start_kill();
        let _ = timeout(Duration::from_millis(250), child.wait()).await;
    }
    result
}

async fn exchange(
    child: &mut tokio::process::Child,
    username: &str,
    secret: Secret,
    attempt_id: u64,
    stream: &mut UnixStream,
    initial_deadline: Duration,
    interactive_deadline: Duration,
) -> Result<PamReply> {
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("PAM helper stdin missing"))?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("PAM helper stdout missing"))?;
    timeout(
        WRITE_DEADLINE,
        write_frame(
            &mut input,
            &HelperRequest::Start {
                username: username.to_owned(),
                secret,
            },
        ),
    )
    .await
    .context("PAM helper start write timed out")??;
    let mut output = BufReader::new(output);
    let started = tokio::time::Instant::now();
    let mut interactive = false;
    let mut sequence = 0_u32;
    let mut notices = 0_u32;
    loop {
        let deadline = if interactive {
            interactive_deadline
        } else {
            initial_deadline
        };
        let remaining = deadline.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            bail!("PAM helper exceeded authentication deadline");
        }
        let message: HelperMessage = timeout(remaining, read_frame(&mut output))
            .await
            .context("PAM helper exceeded authentication deadline")??
            .ok_or_else(|| anyhow!("PAM helper closed without verdict"))?;
        match message {
            HelperMessage::Challenge {
                sequence: next,
                echo,
                text,
            } => {
                if next != sequence + 1
                    || next > protocol::MAX_CHALLENGES
                    || !protocol::valid_text(&text)
                {
                    bail!("PAM helper sent invalid challenge");
                }
                sequence = next;
                interactive = true;
                timeout(
                    WRITE_DEADLINE,
                    ipc::write_daemon_message(
                        stream,
                        &DaemonMessage::AuthenticationChallenge {
                            attempt_id,
                            sequence,
                            echo,
                            text,
                        },
                    ),
                )
                .await
                .context("challenge delivery timed out")??;
                let remaining = interactive_deadline.saturating_sub(started.elapsed());
                let response = timeout(
                    PROMPT_DEADLINE.min(remaining),
                    ipc::read_client_message(stream),
                )
                .await
                .context("authentication challenge timed out")??
                .ok_or_else(|| anyhow!("authentication client closed during challenge"))?;
                match response {
                    ClientMessage::AuthenticationResponse {
                        attempt_id: id,
                        sequence: answer,
                        secret,
                    } if id == attempt_id
                        && answer == sequence
                        && protocol::valid_secret(&secret) =>
                    {
                        timeout(
                            WRITE_DEADLINE,
                            write_frame(&mut input, &HelperRequest::Response { sequence, secret }),
                        )
                        .await
                        .context("PAM response write timed out")??;
                    }
                    ClientMessage::CancelAuthentication {
                        attempt_id: id,
                        sequence: answer,
                    } if id == attempt_id && answer == sequence => {
                        let _ = timeout(
                            WRITE_DEADLINE,
                            write_frame(&mut input, &HelperRequest::Cancel { sequence }),
                        )
                        .await;
                        bail!("authentication cancelled");
                    }
                    _ => bail!("invalid authentication challenge response"),
                }
            }
            HelperMessage::Notice { text } => {
                notices += 1;
                if notices > protocol::MAX_NOTICES || !protocol::valid_text(&text) {
                    bail!("PAM helper sent invalid notice");
                }
                timeout(
                    WRITE_DEADLINE,
                    ipc::write_daemon_message(
                        stream,
                        &DaemonMessage::AuthenticationNotice { attempt_id, text },
                    ),
                )
                .await
                .context("notice delivery timed out")??;
            }
            HelperMessage::Verdict { accepted, message } => {
                if message
                    .as_deref()
                    .is_some_and(|text| !protocol::valid_text(text))
                {
                    bail!("PAM helper sent invalid verdict");
                }
                let status = timeout(WRITE_DEADLINE, child.wait())
                    .await
                    .context("PAM helper exit timed out")??;
                if !status.success() {
                    bail!("PAM helper failed");
                }
                return Ok(PamReply { accepted, message });
            }
        }
    }
}

async fn read_frame<R: AsyncBufRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
) -> Result<Option<T>> {
    let mut frame = FrameBuffer::default();
    loop {
        let available = reader
            .fill_buf()
            .await
            .context("failed to read PAM helper frame")?;
        if available.is_empty() {
            return Ok(frame.end_of_input()?);
        }
        let (consumed, complete) = frame.push(available)?;
        reader.consume(consumed);
        if complete {
            return Ok(Some(frame.decode()?));
        }
    }
}

async fn write_frame<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<()> {
    let frame = protocol::encode_frame(message)?;
    writer
        .write_all(&frame)
        .await
        .context("failed to write PAM helper frame")?;
    writer
        .flush()
        .await
        .context("failed to flush PAM helper frame")
}

#[cfg(test)]
mod tests {
    use std::process::Stdio;

    use super::*;

    #[tokio::test]
    async fn silent_helper_is_killed_at_deadline() {
        let mut child = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn fixture");
        let (mut stream, _peer) = UnixStream::pair().expect("socket pair");
        let result = run_with_cleanup(
            &mut child,
            "test",
            Secret::from(String::from("password")),
            1,
            &mut stream,
            Duration::from_millis(30),
            Duration::from_millis(30),
        )
        .await;
        assert!(result.is_err());
        assert!(child.try_wait().expect("inspect helper").is_some());
    }

    #[tokio::test]
    async fn crashed_helper_cannot_accept_authentication() {
        let mut child = Command::new("false")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn fixture");
        let (mut stream, _peer) = UnixStream::pair().expect("socket pair");
        let result = run_with_cleanup(
            &mut child,
            "test",
            Secret::from(String::from("password")),
            1,
            &mut stream,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .await;
        assert!(result.is_err());
        assert!(child.try_wait().expect("inspect helper").is_some());
    }
}
