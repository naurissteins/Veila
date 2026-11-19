use veila_common::Secret;

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
}

impl AuthEvent {
    pub(crate) fn rejected(attempt_id: u64, message: Option<String>) -> Self {
        Self::Rejected {
            attempt_id,
            retry_after_ms: None,
            failed_attempts: None,
            message,
        }
    }
}

pub(crate) enum ChallengeReply {
    Response { sequence: u32, secret: Secret },
    Cancel { sequence: u32 },
}
