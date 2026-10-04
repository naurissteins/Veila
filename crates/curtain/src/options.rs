use std::path::PathBuf;

use veila_common::{
    BatterySnapshot, NowPlayingSnapshot, WeatherCondition, WeatherSnapshot, ipc::LatencyReportMode,
};

/// Command-line options for the curtain process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurtainOptions {
    pub help: bool,
    pub lock: bool,
    pub force_emergency_ui: bool,
    pub latency_report: LatencyReportMode,
    pub notify_socket: Option<PathBuf>,
    pub daemon_socket: Option<PathBuf>,
    pub control_socket: Option<PathBuf>,
    pub owner_gate: bool,
    pub owner_record: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub initial_background_path: Option<PathBuf>,
    pub preview_png: Option<PathBuf>,
    pub preview_size: Option<veila_renderer::FrameSize>,
    pub preview_artwork: Option<PathBuf>,
    pub preview_title: Option<String>,
    pub preview_artist: Option<String>,
    pub preview_username: Option<String>,
    pub preview_hide_widgets: bool,
    pub preview_hide_weather: bool,
    pub preview_hide_battery: bool,
    pub preview_hide_now_playing: bool,
    pub preview_hide_keyboard_label: bool,
    pub preview_weather_location: Option<String>,
    pub preview_weather_condition: Option<WeatherCondition>,
    pub preview_weather_temperature_celsius: Option<i16>,
    pub preview_battery_percent: Option<u8>,
    pub preview_battery_charging: Option<bool>,
    pub preview_time: Option<PreviewClockTime>,
    pub weather_snapshot: Option<WeatherSnapshot>,
    pub battery_snapshot: Option<BatterySnapshot>,
    pub now_playing_snapshot: Option<NowPlayingSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewClockTime {
    pub hour: u8,
    pub minute: u8,
}

impl CurtainOptions {
    pub(super) fn uses_daemon_lock_flow(&self) -> bool {
        self.notify_socket.is_some()
            || self.daemon_socket.is_some()
            || self.control_socket.is_some()
            || self.owner_record.is_some()
            || self.owner_gate
    }
}
