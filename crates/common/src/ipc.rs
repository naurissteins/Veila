use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Messages sent from UI-facing clients to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientMessage {
    SubmitPassword { attempt_id: u64, secret: String },
    CancelAuthentication,
}

/// Messages sent from the daemon to UI-facing clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonMessage {
    AuthenticationAccepted {
        attempt_id: u64,
    },
    AuthenticationRejected {
        attempt_id: u64,
        retry_after_ms: Option<u64>,
    },
    AuthenticationBusy {
        attempt_id: u64,
    },
}

/// Messages sent from the daemon to the secure curtain process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CurtainControlMessage {
    Unlock { attempt_id: Option<u64> },
}

/// Messages sent to the long-running daemon control socket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonControlMessage {
    LockNow,
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonStatus {
    pub state: String,
    pub session: String,
    pub curtain_running: bool,
    pub config_path: Option<String>,
}

/// Responses sent by the long-running daemon control socket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonControlResponse {
    Accepted,
    Status(DaemonStatus),
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
    use super::{
        ClientMessage, CurtainControlMessage, DaemonControlMessage, DaemonControlResponse,
        DaemonStatus, decode_message, encode_message,
    };

    #[test]
    fn round_trips_json_messages() {
        let message = ClientMessage::CancelAuthentication;
        let encoded = encode_message(&message).expect("ipc message should encode");
        let decoded = decode_message::<ClientMessage>(&encoded).expect("ipc message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_control_messages() {
        let message = CurtainControlMessage::Unlock {
            attempt_id: Some(7),
        };
        let encoded = encode_message(&message).expect("control message should encode");
        let decoded = decode_message::<CurtainControlMessage>(&encoded)
            .expect("control message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_daemon_control_messages() {
        let message = DaemonControlMessage::Status;
        let encoded = encode_message(&message).expect("daemon control message should encode");
        let decoded = decode_message::<DaemonControlMessage>(&encoded)
            .expect("daemon control message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_daemon_control_responses() {
        let message = DaemonControlResponse::Status(DaemonStatus {
            state: "locked".to_string(),
            session: "/org/freedesktop/login1/session/_32".to_string(),
            curtain_running: true,
            config_path: Some("/tmp/kwylock.toml".to_string()),
        });
        let encoded = encode_message(&message).expect("daemon control response should encode");
        let decoded = decode_message::<DaemonControlResponse>(&encoded)
            .expect("daemon control response should decode");

        assert_eq!(decoded, message);
    }
}
