use std::{
    ffi::{OsStr, OsString},
    io::{BufRead, BufReader, Write},
    sync::{Arc, Mutex},
};

use anyhow::{Result, bail};
use nonstick::{
    AuthnFlags, ConversationAdapter, ErrorCode, Result as PamResult, Transaction,
    TransactionBuilder,
};
use veila_common::Secret;

use super::{
    protocol::{self, HelperMessage, HelperRequest},
    service,
};

struct ConversationState {
    input: Box<dyn BufRead + Send>,
    output: Box<dyn Write + Send>,
    initial_secret: Option<Secret>,
    sequence: u32,
    notices: u32,
}

struct InteractiveConversation {
    state: Arc<Mutex<ConversationState>>,
    last_message: Arc<Mutex<Option<String>>>,
}

impl InteractiveConversation {
    fn respond(&self, request: &OsStr, echo: bool) -> PamResult<OsString> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ErrorCode::ConversationError)?;
        if state.notices > protocol::MAX_NOTICES {
            return Err(ErrorCode::ConversationError);
        }
        if !echo && let Some(secret) = state.initial_secret.take() {
            return Ok(OsString::from(secret.expose()));
        }
        state.sequence += 1;
        if state.sequence > protocol::MAX_CHALLENGES {
            return Err(ErrorCode::ConversationError);
        }
        let sequence = state.sequence;
        let text = protocol::sanitize_text(&request.to_string_lossy());
        // Each later prompt consumes exactly one fresh answer from the curtain.
        let ConversationState { input, output, .. } = &mut *state;
        protocol::write_sync(
            output,
            &HelperMessage::Challenge {
                sequence,
                echo,
                text,
            },
        )
        .map_err(|_| ErrorCode::ConversationError)?;
        match protocol::read_sync::<_, HelperRequest>(input)
            .map_err(|_| ErrorCode::ConversationError)?
        {
            Some(HelperRequest::Response {
                sequence: answer,
                secret,
            }) if answer == sequence && protocol::valid_secret(&secret) => {
                Ok(OsString::from(secret.expose()))
            }
            _ => Err(ErrorCode::ConversationError),
        }
    }

    fn notice(&self, message: &OsStr) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let text = protocol::sanitize_text(&message.to_string_lossy());
        if text.is_empty() {
            return;
        }
        if let Ok(mut last) = self.last_message.lock() {
            *last = Some(text.clone());
        }
        state.notices += 1;
        if state.notices <= protocol::MAX_NOTICES {
            let _ = protocol::write_sync(&mut state.output, &HelperMessage::Notice { text });
        }
    }
}

impl ConversationAdapter for InteractiveConversation {
    fn prompt(&self, request: impl AsRef<OsStr>) -> PamResult<OsString> {
        self.respond(request.as_ref(), true)
    }

    fn masked_prompt(&self, request: impl AsRef<OsStr>) -> PamResult<OsString> {
        self.respond(request.as_ref(), false)
    }

    fn error_msg(&self, message: impl AsRef<OsStr>) {
        self.notice(message.as_ref());
    }
    fn info_msg(&self, message: impl AsRef<OsStr>) {
        self.notice(message.as_ref());
    }
}

fn pam_service() -> Result<String> {
    #[cfg(debug_assertions)]
    if let Ok(service) = std::env::var("VEILA_PAM_SERVICE") {
        tracing::warn!(service, "using debug PAM service override");
        return Ok(service);
    }
    let Some(selected) = service::selected_service() else {
        bail!("no Veila PAM service or supported fallback exists in /etc/pam.d");
    };
    if selected.fallback {
        tracing::warn!(
            service = selected.name,
            "Veila PAM service missing; using fallback"
        );
    }
    Ok(selected.name.to_owned())
}

pub(super) fn run_helper() -> Result<()> {
    let mut input = BufReader::new(std::io::stdin());
    let Some(HelperRequest::Start { username, secret }) = protocol::read_sync(&mut input)? else {
        bail!("PAM helper expected start request");
    };
    if username.is_empty() || username.len() > 256 || !protocol::valid_secret(&secret) {
        bail!("invalid PAM helper start request");
    }
    let last_message = Arc::new(Mutex::new(None));
    let state = Arc::new(Mutex::new(ConversationState {
        input: Box::new(input),
        output: Box::new(std::io::stdout()),
        initial_secret: Some(secret),
        sequence: 0,
        notices: 0,
    }));
    let conversation = InteractiveConversation {
        state: Arc::clone(&state),
        last_message: Arc::clone(&last_message),
    };
    let service = match pam_service() {
        Ok(service) => service,
        Err(error) => {
            tracing::error!("PAM service selection failed: {error:#}");
            return protocol::write_sync(
                &mut std::io::stdout(),
                &HelperMessage::Verdict {
                    accepted: false,
                    message: Some(String::from("PAM service unavailable; run veila doctor")),
                },
            );
        }
    };
    let result = TransactionBuilder::new_with_service(&service)
        .username(&username)
        .build(conversation.into_conversation())
        .and_then(|mut transaction| transaction.authenticate(AuthnFlags::empty()));
    let message = last_message.lock().ok().and_then(|last| last.clone());
    let notice_overflow = state
        .lock()
        .map_or(true, |state| state.notices > protocol::MAX_NOTICES);
    protocol::write_sync(
        &mut std::io::stdout(),
        &HelperMessage::Verdict {
            accepted: result.is_ok() && !notice_overflow,
            message,
        },
    )
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use super::*;

    struct SharedOutput(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedOutput {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("output lock").extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn conversation(
        responses: Vec<HelperRequest>,
    ) -> (InteractiveConversation, Arc<Mutex<Vec<u8>>>) {
        let mut input = Vec::new();
        for response in responses {
            protocol::write_sync(&mut input, &response).expect("encode response");
        }
        let output = Arc::new(Mutex::new(Vec::new()));
        (
            InteractiveConversation {
                state: Arc::new(Mutex::new(ConversationState {
                    input: Box::new(Cursor::new(input)),
                    output: Box::new(SharedOutput(Arc::clone(&output))),
                    initial_secret: Some(Secret::from(String::from("password"))),
                    sequence: 0,
                    notices: 0,
                })),
                last_message: Arc::new(Mutex::new(None)),
            },
            output,
        )
    }

    #[test]
    fn first_hidden_prompt_uses_initial_password_only_once() {
        let (conversation, output) = conversation(vec![
            HelperRequest::Response {
                sequence: 1,
                secret: Secret::from(String::from("123456")),
            },
            HelperRequest::Response {
                sequence: 2,
                secret: Secret::from(String::from("yes")),
            },
        ]);
        assert_eq!(
            conversation.masked_prompt("Password:").expect("password"),
            "password"
        );
        assert_eq!(conversation.masked_prompt("Code:").expect("code"), "123456");
        assert_eq!(conversation.prompt("Continue?").expect("visible"), "yes");
        let frames = String::from_utf8(output.lock().expect("output").clone()).expect("utf8");
        assert!(frames.contains("\"sequence\":1"));
        assert!(frames.contains("\"sequence\":2"));
        assert!(frames.contains("\"echo\":true"));
        assert!(!frames.contains("password"));
        assert!(!frames.contains("123456"));
    }

    #[test]
    fn cancellation_fails_conversation() {
        let (conversation, _) = conversation(vec![HelperRequest::Cancel { sequence: 1 }]);
        conversation.masked_prompt("Password:").expect("password");
        assert!(conversation.masked_prompt("Code:").is_err());
    }

    #[test]
    fn stale_response_fails_conversation() {
        let (conversation, _) = conversation(vec![HelperRequest::Response {
            sequence: 2,
            secret: Secret::from(String::from("code")),
        }]);
        conversation.masked_prompt("Password:").expect("password");
        assert!(conversation.masked_prompt("Code:").is_err());
    }

    #[test]
    fn notice_overflow_fails_conversation() {
        let (conversation, output) = conversation(Vec::new());
        for _ in 0..=protocol::MAX_NOTICES {
            conversation.info_msg("Info");
        }
        assert!(conversation.masked_prompt("Password:").is_err());
        assert_eq!(
            String::from_utf8(output.lock().expect("output").clone())
                .expect("utf8")
                .lines()
                .count(),
            protocol::MAX_NOTICES as usize
        );
    }
}
