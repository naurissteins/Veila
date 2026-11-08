use std::{io, sync::mpsc, thread, time::Instant};

use calloop::channel::Sender;
use nix::unistd::{Uid, User};
use veila_auth::{Challenge, Conversation};
use veila_common::Secret;

use crate::ipc::auth::{AuthEvent, ChallengeReply};

pub(crate) fn submit_password(
    attempt_id: u64,
    secret: Secret,
    sender: Sender<AuthEvent>,
) -> io::Result<mpsc::Sender<ChallengeReply>> {
    let (reply_sender, replies) = mpsc::channel();
    thread::Builder::new()
        .name(String::from("veila-auth"))
        .spawn(move || {
            let mut conversation = Relay {
                attempt_id,
                sender,
                replies,
            };
            let username = User::from_uid(Uid::current());
            let event = match username {
                Ok(Some(user)) => {
                    // The spawning thread must live until authenticate has reaped its helper.
                    match veila_auth::authenticate(&user.name, secret, &mut conversation) {
                        Ok(verdict) if verdict.accepted => AuthEvent::Accepted { attempt_id },
                        Ok(verdict) => AuthEvent::Rejected {
                            attempt_id,
                            retry_after_ms: None,
                            failed_attempts: None,
                            message: verdict.message,
                        },
                        Err(error) => {
                            tracing::warn!(attempt_id, %error, "curtain PAM attempt failed");
                            AuthEvent::Failed { attempt_id }
                        }
                    }
                }
                _ => {
                    tracing::warn!(attempt_id, "failed to resolve the current uid for PAM");
                    AuthEvent::Failed { attempt_id }
                }
            };
            let _ = conversation.sender.send(event);
        })?;
    Ok(reply_sender)
}

struct Relay {
    attempt_id: u64,
    sender: Sender<AuthEvent>,
    replies: mpsc::Receiver<ChallengeReply>,
}

impl Conversation for Relay {
    fn notice(&mut self, text: String) {
        let _ = self.sender.send(AuthEvent::Notice {
            attempt_id: self.attempt_id,
            text,
        });
    }

    fn challenge(&mut self, challenge: Challenge, deadline: Instant) -> Option<Secret> {
        let sequence = challenge.sequence;
        self.sender
            .send(AuthEvent::Challenge {
                attempt_id: self.attempt_id,
                sequence,
                echo: challenge.echo,
                text: challenge.text,
            })
            .ok()?;
        match self
            .replies
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .ok()?
        {
            ChallengeReply::Response {
                sequence: answer,
                secret,
            } if answer == sequence => Some(secret),
            ChallengeReply::Cancel { sequence: answer } if answer == sequence => None,
            // stale replies and a disconnected UI fail this attempt closed
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
