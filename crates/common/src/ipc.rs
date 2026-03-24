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
    /// Tells a standby curtain to acquire the session lock immediately.
    LockNow {
        notify_socket: String,
        daemon_socket: String,
    },
    Unlock {
        attempt_id: Option<u64>,
    },
    ReloadConfig,
}

/// Messages sent to the long-running daemon control socket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonControlMessage {
    LockNow,
    Stop,
    Status,
    Health,
    ReloadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonStatus {
    pub state: String,
    pub session: String,
    pub active_lock: bool,
    pub curtain_running: bool,
    pub live_reload_available: bool,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonHealth {
    pub component: String,
    pub version: String,
    pub build_profile: String,
    pub target_os: String,
    pub target_arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonReloadStatus {
    pub config_path: Option<String>,
    pub active_lock: bool,
    pub live_reload: LiveReloadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiveReloadStatus {
    NotActive,
    Forwarded,
}

/// Responses sent by the long-running daemon control socket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonControlResponse {
    Accepted,
    Status(DaemonStatus),
    Health(DaemonHealth),
    Reloaded(DaemonReloadStatus),
    Error { reason: String },
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
        DaemonHealth, DaemonReloadStatus, DaemonStatus, LiveReloadStatus, decode_message,
        encode_message,
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
    fn round_trips_lock_now_message() {
        let message = CurtainControlMessage::LockNow {
            notify_socket: "/tmp/veila-notify.sock".to_string(),
            daemon_socket: "/tmp/veila-auth.sock".to_string(),
        };
        let encoded = encode_message(&message).expect("lock-now message should encode");
        let decoded = decode_message::<CurtainControlMessage>(&encoded)
            .expect("lock-now message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_daemon_control_messages() {
        let message = DaemonControlMessage::Stop;
        let encoded = encode_message(&message).expect("daemon control message should encode");
        let decoded = decode_message::<DaemonControlMessage>(&encoded)
            .expect("daemon control message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_daemon_control_responses() {
        let message = DaemonControlResponse::Health(DaemonHealth {
            component: "veilad".to_string(),
            version: "0.1.0".to_string(),
            build_profile: "debug".to_string(),
            target_os: "linux".to_string(),
            target_arch: "x86_64".to_string(),
        });
        let encoded = encode_message(&message).expect("daemon control response should encode");
        let decoded = decode_message::<DaemonControlResponse>(&encoded)
            .expect("daemon control response should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_reload_status_response() {
        let message = DaemonControlResponse::Reloaded(DaemonReloadStatus {
            config_path: Some("/tmp/veila.toml".to_string()),
            active_lock: true,
            live_reload: LiveReloadStatus::Forwarded,
        });
        let encoded = encode_message(&message).expect("daemon control response should encode");
        let decoded = decode_message::<DaemonControlResponse>(&encoded)
            .expect("daemon control response should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_status_response() {
        let message = DaemonControlResponse::Status(DaemonStatus {
            state: "locked".to_string(),
            session: "/org/freedesktop/login1/session/_3".to_string(),
            active_lock: true,
            curtain_running: true,
            live_reload_available: true,
            config_path: Some("/tmp/veila.toml".to_string()),
        });
        let encoded = encode_message(&message).expect("daemon control response should encode");
        let decoded = decode_message::<DaemonControlResponse>(&encoded)
            .expect("daemon control response should decode");

        assert_eq!(decoded, message);
    }
}
