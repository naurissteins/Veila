pub mod fd;
pub mod frame;

use crate::{
    AppConfig, BatterySnapshot, PowerAction, WeatherSnapshot,
    ipc::{CurtainControlMessage, CurtainLatencyReport},
};
use serde::{Deserialize, Serialize};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const EXIT_UNLOCKED: i32 = 0;
pub const EXIT_REFUSED: i32 = 2;
pub const EXIT_ABNORMAL: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum SupervisorMessage {
    Control(CurtainControlMessage),
    ArmResumeInputGuard { request: u64 },
    ReloadConfig { config: Box<AppConfig> },
    UpdateWeather { snapshot: Option<WeatherSnapshot> },
    UpdateBattery { snapshot: Option<BatterySnapshot> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum CurtainMessage {
    SessionLocked,
    Ready {
        latency_report: Option<Box<CurtainLatencyReport>>,
    },
    ResumeGuardArmed {
        request: u64,
    },
    Activity,
    AuthInFlight(bool),
    RequestPowerAction {
        action: PowerAction,
    },
}
