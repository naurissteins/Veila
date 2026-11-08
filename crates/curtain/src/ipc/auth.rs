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
    /// the attempt produced no verdict; the helper failed, closed, or exceeded its deadline
    Failed {
        attempt_id: u64,
    },
}

pub(crate) enum ChallengeReply {
    Response { sequence: u32, secret: Secret },
    Cancel { sequence: u32 },
}
