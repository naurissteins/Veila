pub mod line;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::{BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot};

pub use line::{IPC_MAX_LINE_BYTES, LineAccumulator, LineProgress};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurtainInitialSnapshots {
    pub weather: Option<WeatherSnapshot>,
    pub battery: Option<BatterySnapshot>,
    pub now_playing: Option<NowPlayingSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockPowerStatusSnapshot {
    pub suspend_remaining_seconds: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FingerprintStatus {
    Ready,
    Scanning,
    Accepted,
    NotRecognized,
    AttemptLimitReached,
    NoEnrolledFingers,
    Unavailable,
    Error,
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
    pub placeholder_committed_us: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CurtainStartupMessage {
    SessionLocked,
    Ready {
        latency_report: Option<Box<CurtainLatencyReport>>,
    },
}

/// control updates delivered over the inherited supervisor channel
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum CurtainControlMessage {
    Unlock {
        attempt_id: Option<u64>,
    },
    ArmResumeInputGuard,
    MarkResumed,
    UpdateNowPlaying {
        snapshot: Option<NowPlayingSnapshot>,
    },
    UpdatePowerStatus {
        snapshot: Option<LockPowerStatusSnapshot>,
    },
    UpdateFingerprintStatus {
        status: Option<FingerprintStatus>,
    },
}

/// Encodes an IPC message as JSON for the initial control channel
pub fn encode_message<T>(message: &T) -> Result<String>
where
    T: Serialize,
{
    serde_json::to_string(message).map_err(Into::into)
}

/// Decodes an IPC message from JSON for the initial control channel
pub fn decode_message<T>(input: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(input).map_err(Into::into)
}
