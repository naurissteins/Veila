use std::{
    io::{BufRead, BufReader},
    os::fd::AsFd,
    process::{Child, ChildStdout, Command, ExitStatus},
    time::{Duration, Instant},
};

use nix::{
    errno::Errno,
    poll::{PollFd, PollFlags, PollTimeout, poll},
};
use serde::de::DeserializeOwned;
use veila_common::Secret;

use crate::{
    helper::helper_command,
    protocol::{
        self, FrameBuffer, HelperMessage, HelperRequest, INITIAL_DEADLINE, INTERACTIVE_DEADLINE,
        MAX_CHALLENGES, MAX_NOTICES, PROMPT_DEADLINE, ProtocolError, WRITE_DEADLINE,
    },
};

const KILL_REAP_DEADLINE: Duration = Duration::from_millis(250);
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub sequence: u32,
    pub echo: bool,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub accepted: bool,
    pub message: Option<String>,
}

/// Relays helper prompts to the user. Implementations run on the authentication worker thread.
pub trait Conversation {
    fn notice(&mut self, text: String);

    /// Returns the answer, or `None` to cancel the attempt. An answer after `deadline` fails the attempt.
    fn challenge(&mut self, challenge: Challenge, deadline: Instant) -> Option<Secret>;
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("failed to start PAM helper")]
    Spawn(#[source] std::io::Error),
    #[error("PAM helper pipes are missing")]
    MissingPipe,
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("failed to wait for PAM helper output")]
    Poll(#[source] Errno),
    #[error("failed to wait for PAM helper exit")]
    Wait(#[source] std::io::Error),
    #[error("PAM helper exceeded authentication deadline")]
    Deadline,
    #[error("PAM helper closed without verdict")]
    ClosedWithoutVerdict,
    #[error("PAM helper sent invalid challenge")]
    InvalidChallenge,
    #[error("PAM helper sent invalid notice")]
    InvalidNotice,
    #[error("PAM helper sent invalid verdict")]
    InvalidVerdict,
    #[error("invalid authentication challenge response")]
    InvalidResponse,
    #[error("authentication challenge timed out")]
    ChallengeTimeout,
    #[error("authentication cancelled")]
    Cancelled,
    #[error("PAM helper exit timed out")]
    ExitTimeout,
    #[error("PAM helper failed")]
    HelperFailed,
}

#[derive(Debug, Clone, Copy)]
struct Deadlines {
    initial: Duration,
    interactive: Duration,
    prompt: Duration,
}

impl Default for Deadlines {
    fn default() -> Self {
        Self {
            initial: INITIAL_DEADLINE,
            interactive: INTERACTIVE_DEADLINE,
            prompt: PROMPT_DEADLINE,
        }
    }
}

/// Runs one PAM attempt in a fresh helper process and blocks until its verdict.
///
/// Any error means the attempt failed; the helper is killed and reaped before returning.
pub fn authenticate(
    username: &str,
    secret: Secret,
    conversation: &mut impl Conversation,
) -> Result<Verdict, ClientError> {
    run(
        helper_command(),
        username,
        secret,
        conversation,
        Deadlines::default(),
    )
}

fn run(
    mut command: Command,
    username: &str,
    secret: Secret,
    conversation: &mut impl Conversation,
    deadlines: Deadlines,
) -> Result<Verdict, ClientError> {
    let mut child = command.spawn().map_err(ClientError::Spawn)?;
    let result = exchange(&mut child, username, secret, conversation, deadlines);
    if result.is_err() {
        let _ = child.kill();
        let _ = wait_until(&mut child, Instant::now() + KILL_REAP_DEADLINE);
    }
    result
}

fn exchange(
    child: &mut Child,
    username: &str,
    secret: Secret,
    conversation: &mut impl Conversation,
    deadlines: Deadlines,
) -> Result<Verdict, ClientError> {
    let mut input = child.stdin.take().ok_or(ClientError::MissingPipe)?;
    let output = child.stdout.take().ok_or(ClientError::MissingPipe)?;
    // Every request follows a helper message that drained the previous request, so these
    // writes fit the empty pipe and never block.
    protocol::write_frame(
        &mut input,
        &HelperRequest::Start {
            username: username.to_owned(),
            secret,
        },
    )?;
    let mut output = BufReader::new(output);
    let started = Instant::now();
    let mut interactive = false;
    let mut sequence = 0_u32;
    let mut notices = 0_u32;
    loop {
        let limit = if interactive {
            deadlines.interactive
        } else {
            deadlines.initial
        };
        let message: HelperMessage =
            read_before(&mut output, started + limit)?.ok_or(ClientError::ClosedWithoutVerdict)?;
        match message {
            HelperMessage::Challenge {
                sequence: next,
                echo,
                text,
            } => {
                if next != sequence + 1 || next > MAX_CHALLENGES || !protocol::valid_text(&text) {
                    return Err(ClientError::InvalidChallenge);
                }
                sequence = next;
                interactive = true;
                let deadline =
                    (Instant::now() + deadlines.prompt).min(started + deadlines.interactive);
                let answer = conversation.challenge(
                    Challenge {
                        sequence,
                        echo,
                        text,
                    },
                    deadline,
                );
                if Instant::now() > deadline {
                    return Err(ClientError::ChallengeTimeout);
                }
                match answer {
                    Some(secret) if protocol::valid_secret(&secret) => {
                        protocol::write_frame(
                            &mut input,
                            &HelperRequest::Response { sequence, secret },
                        )?;
                    }
                    Some(_) => return Err(ClientError::InvalidResponse),
                    None => {
                        let _ =
                            protocol::write_frame(&mut input, &HelperRequest::Cancel { sequence });
                        return Err(ClientError::Cancelled);
                    }
                }
            }
            HelperMessage::Notice { text } => {
                notices += 1;
                if notices > MAX_NOTICES || !protocol::valid_text(&text) {
                    return Err(ClientError::InvalidNotice);
                }
                conversation.notice(text);
            }
            HelperMessage::Verdict { accepted, message } => {
                if message
                    .as_deref()
                    .is_some_and(|text| !protocol::valid_text(text))
                {
                    return Err(ClientError::InvalidVerdict);
                }
                let status = wait_until(child, Instant::now() + WRITE_DEADLINE)?
                    .ok_or(ClientError::ExitTimeout)?;
                if !status.success() {
                    return Err(ClientError::HelperFailed);
                }
                return Ok(Verdict { accepted, message });
            }
        }
    }
}

fn read_before<T: DeserializeOwned>(
    reader: &mut BufReader<ChildStdout>,
    deadline: Instant,
) -> Result<Option<T>, ClientError> {
    let mut frame = FrameBuffer::default();
    loop {
        if reader.buffer().is_empty() {
            wait_readable(reader.get_ref(), deadline)?;
        }
        let available = reader.fill_buf().map_err(ProtocolError::Read)?;
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

fn wait_readable(fd: &impl AsFd, deadline: Instant) -> Result<(), ClientError> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ClientError::Deadline);
        }
        // Round up so a sub-millisecond remainder does not become a busy zero-timeout poll.
        let timeout = PollTimeout::try_from(remaining.as_millis().saturating_add(1))
            .unwrap_or(PollTimeout::MAX);
        let mut fds = [PollFd::new(fd.as_fd(), PollFlags::POLLIN)];
        match poll(&mut fds, timeout) {
            Ok(0) | Err(Errno::EINTR) => {}
            Ok(_) => return Ok(()),
            Err(error) => return Err(ClientError::Poll(error)),
        }
    }
}

fn wait_until(child: &mut Child, deadline: Instant) -> Result<Option<ExitStatus>, ClientError> {
    loop {
        if let Some(status) = child.try_wait().map_err(ClientError::Wait)? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        std::thread::sleep(EXIT_POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests;
