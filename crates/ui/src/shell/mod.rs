mod avatar;
mod battery;
mod clock;
mod emergency;
mod input;
mod now_playing;
mod pointer;
mod render;
mod state;
#[cfg(test)]
mod tests;
mod theme;
mod weather;

pub use avatar::{load_avatar, load_cached_avatar};
pub use theme::ShellTheme;

use std::{cell::RefCell, time::Instant};

use battery::BatteryWidgetData;
use clock::ClockState;
use now_playing::NowPlayingWidgetData;
use render::TextLayoutCache;
use veila_common::{FingerprintStatus, PowerAction, Secret};
use veila_renderer::avatar::AvatarAsset;
use weather::WeatherWidgetData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    None,
    Submit(Secret),
    Power(PowerAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAnimationUpdate {
    None,
    AuthDirty,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKey {
    Character(char),
    Backspace,
    Enter,
    Escape,
    Clear,
    SelectAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShellStatus {
    Idle,
    Pending {
        started_at: Instant,
        visible_after: Instant,
        shown: bool,
    },
    Rejected {
        retry_until: Option<Instant>,
        displayed_retry_seconds: Option<u64>,
        failed_attempts: Option<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellMode {
    Rich,
    Emergency,
}

#[derive(Debug, Clone)]
struct NowPlayingTransition {
    previous: Option<NowPlayingWidgetData>,
    started_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PowerConfirmation {
    action: PowerAction,
    expires_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewGrid {
    pub cell_size: i32,
    pub color: veila_renderer::ClearColor,
    pub major_every: i32,
    pub major_color: veila_renderer::ClearColor,
}

#[derive(Debug, Clone)]
pub struct ShellState {
    mode: ShellMode,
    secret: Secret,
    secret_selected: bool,
    caps_lock_active: bool,
    keyboard_layout_label: Option<String>,
    battery: Option<BatteryWidgetData>,
    power_status_text: Option<String>,
    fingerprint_status: Option<FingerprintStatus>,
    reveal_secret: bool,
    auth_revealed: bool,
    reveal_toggle_hovered: bool,
    reveal_toggle_pressed: bool,
    power_button_hovered: Option<PowerAction>,
    power_button_pressed: Option<PowerAction>,
    power_confirmation: Option<PowerConfirmation>,
    requested_power_action: Option<PowerAction>,
    static_scene_revision: u64,
    focused: bool,
    status: ShellStatus,
    clock: ClockState,
    theme: ShellTheme,
    hint_text: String,
    reveal_hint_text: String,
    username_text: Option<String>,
    weather: Option<WeatherWidgetData>,
    now_playing: Option<NowPlayingWidgetData>,
    now_playing_transition: Option<NowPlayingTransition>,
    avatar: AvatarAsset,
    preview_grid_enabled: bool,
    text_layout_cache: RefCell<TextLayoutCache>,
    render_scale: u32,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new(ShellTheme::default(), None, None, true)
    }
}
