use serde::{Deserialize, Serialize};

use crate::NowPlayingSnapshot;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockPowerStatusSnapshot {
    pub suspend_remaining_seconds: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum LatencyReportMode {
    #[default]
    Disabled,
    Basic,
    Verbose,
}

impl LatencyReportMode {
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub const fn is_verbose(self) -> bool {
        matches!(self, Self::Verbose)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurtainLatencyReport {
    pub wayland_connect_ms: u64,
    pub wayland_connect_us: u64,
    pub registry_ms: u64,
    pub registry_us: u64,
    pub event_loop_ms: u64,
    pub event_loop_us: u64,
    pub app_init_ms: u64,
    pub app_init_us: u64,
    pub lock_request_ms: u64,
    pub lock_request_us: u64,
    pub startup_prepared_ms: u64,
    pub startup_prepared_us: u64,
    pub first_surface_configured_ms: Option<u64>,
    pub first_surface_configured_us: Option<u64>,
    pub all_surfaces_configured_ms: Option<u64>,
    pub all_surfaces_configured_us: Option<u64>,
    pub session_locked_ms: Option<u64>,
    pub session_locked_us: Option<u64>,
    pub first_frame_ms: Option<u64>,
    pub first_frame_us: Option<u64>,
    pub ready_notified_ms: Option<u64>,
    pub ready_notified_us: Option<u64>,
    pub surface_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockLatencyReport {
    pub daemon_config_load_ms: u64,
    pub daemon_config_load_us: u64,
    pub socket_setup_ms: u64,
    pub socket_setup_us: u64,
    pub curtain_spawn_ms: u64,
    pub curtain_spawn_us: u64,
    pub curtain_ready_wait_ms: u64,
    pub curtain_ready_wait_us: u64,
    pub activation_total_ms: u64,
    pub activation_total_us: u64,
    pub curtain: Option<CurtainLatencyReport>,
}

/// Messages sent from UI-facing clients to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientMessage {
    SubmitPassword { attempt_id: u64, secret: String },
    CancelAuthentication,
    Activity,
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
        failed_attempts: Option<u8>,
    },
    AuthenticationBusy {
        attempt_id: u64,
    },
}

/// Messages sent from the daemon to the secure curtain process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CurtainControlMessage {
    Unlock {
        attempt_id: Option<u64>,
    },
    ReloadConfig,
    ArmResumeInputGuard,
    MarkResumed,
    UpdateNowPlaying {
        snapshot: Option<NowPlayingSnapshot>,
    },
    UpdatePowerStatus {
        snapshot: Option<LockPowerStatusSnapshot>,
    },
}

/// Messages sent to the long-running daemon control socket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonControlMessage {
    LockNow {
        wait_ready: bool,
        force_emergency_ui: bool,
        latency_report: LatencyReportMode,
    },
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
    pub auto_reload_enabled: bool,
    pub auto_reload_debounce_ms: u64,
    pub last_reload_result: Option<String>,
    pub last_reload_unix_ms: Option<u64>,
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
    pub reload_source: String,
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
    Locked {
        already_active: bool,
        latency_report: Option<Box<LockLatencyReport>>,
    },
    Status(DaemonStatus),
    Health(DaemonHealth),
    Reloaded(DaemonReloadStatus),
    Error {
        reason: String,
    },
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
        ClientMessage, CurtainControlMessage, CurtainLatencyReport, DaemonControlMessage,
        DaemonControlResponse, DaemonMessage, DaemonReloadStatus, DaemonStatus, LatencyReportMode,
        LiveReloadStatus, LockLatencyReport, LockPowerStatusSnapshot, decode_message,
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
    fn round_trips_auth_rejected_messages_with_failed_attempts() {
        let message = DaemonMessage::AuthenticationRejected {
            attempt_id: 7,
            retry_after_ms: Some(1_500),
            failed_attempts: Some(2),
        };
        let encoded = encode_message(&message).expect("daemon message should encode");
        let decoded =
            decode_message::<DaemonMessage>(&encoded).expect("daemon message should decode");

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
    fn round_trips_now_playing_update_control_messages() {
        let message = CurtainControlMessage::UpdateNowPlaying {
            snapshot: Some(crate::NowPlayingSnapshot {
                title: "Track".to_string(),
                artist: Some("Artist".to_string()),
                artwork_path: None,
                fetched_at_unix: 1,
            }),
        };
        let encoded = encode_message(&message).expect("control message should encode");
        let decoded = decode_message::<CurtainControlMessage>(&encoded)
            .expect("control message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_power_status_update_control_messages() {
        let message = CurtainControlMessage::UpdatePowerStatus {
            snapshot: Some(LockPowerStatusSnapshot {
                suspend_remaining_seconds: 42,
            }),
        };
        let encoded = encode_message(&message).expect("control message should encode");
        let decoded = decode_message::<CurtainControlMessage>(&encoded)
            .expect("control message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_daemon_control_messages() {
        let message = DaemonControlMessage::LockNow {
            wait_ready: true,
            force_emergency_ui: true,
            latency_report: LatencyReportMode::Verbose,
        };
        let encoded = encode_message(&message).expect("daemon control message should encode");
        let decoded = decode_message::<DaemonControlMessage>(&encoded)
            .expect("daemon control message should decode");

        assert_eq!(decoded, message);
    }

    #[test]
    fn round_trips_daemon_control_responses() {
        let message = DaemonControlResponse::Locked {
            already_active: false,
            latency_report: Some(Box::new(LockLatencyReport {
                daemon_config_load_ms: 1,
                daemon_config_load_us: 1001,
                socket_setup_ms: 2,
                socket_setup_us: 2002,
                curtain_spawn_ms: 3,
                curtain_spawn_us: 3003,
                curtain_ready_wait_ms: 4,
                curtain_ready_wait_us: 4004,
                activation_total_ms: 5,
                activation_total_us: 5005,
                curtain: Some(CurtainLatencyReport {
                    wayland_connect_ms: 6,
                    wayland_connect_us: 6006,
                    registry_ms: 7,
                    registry_us: 7007,
                    event_loop_ms: 8,
                    event_loop_us: 8008,
                    app_init_ms: 9,
                    app_init_us: 9009,
                    lock_request_ms: 10,
                    lock_request_us: 10010,
                    startup_prepared_ms: 11,
                    startup_prepared_us: 11011,
                    first_surface_configured_ms: Some(12),
                    first_surface_configured_us: Some(12012),
                    all_surfaces_configured_ms: Some(13),
                    all_surfaces_configured_us: Some(13013),
                    session_locked_ms: Some(14),
                    session_locked_us: Some(14014),
                    first_frame_ms: Some(15),
                    first_frame_us: Some(15015),
                    ready_notified_ms: Some(16),
                    ready_notified_us: Some(16016),
                    surface_count: 2,
                }),
            })),
        };
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
            reload_source: "manual".to_string(),
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
            auto_reload_enabled: true,
            auto_reload_debounce_ms: 250,
            last_reload_result: Some("ok:config-change".to_string()),
            last_reload_unix_ms: Some(1_744_000_000_000),
            config_path: Some("/tmp/veila.toml".to_string()),
        });
        let encoded = encode_message(&message).expect("daemon control response should encode");
        let decoded = decode_message::<DaemonControlResponse>(&encoded)
            .expect("daemon control response should decode");

        assert_eq!(decoded, message);
    }
}
