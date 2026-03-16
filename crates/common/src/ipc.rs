use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Messages sent from UI-facing clients to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientMessage {
    SubmitPassword { secret: String },
    CancelAuthentication,
}

/// Messages sent from the daemon to UI-facing clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonMessage {
    AuthenticationRejected { retry_after_ms: Option<u64> },
    AuthenticationBusy,
}

/// Encodes an IPC message as JSON for the initial control channel.
pub fn encode_message<T>(message: &T) -> Result<String>
where
    T: Serialize,
{
    serde_json::to_string(message).map_err(Into::into)
}

/// Decodes an IPC message from JSON for the initial control channel.
pub fn decode_message<T>(input: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(input).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{ClientMessage, decode_message, encode_message};

    #[test]
    fn round_trips_json_messages() {
        let message = ClientMessage::CancelAuthentication;
        let encoded = encode_message(&message).expect("ipc message should encode");
        let decoded = decode_message::<ClientMessage>(&encoded).expect("ipc message should decode");

        assert_eq!(decoded, message);
    }
}
