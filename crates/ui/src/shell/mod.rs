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

pub use avatar::{has_avatar_candidate, load_avatar, load_cached_avatar};
pub use theme::ShellTheme;

use std::{cell::RefCell, collections::HashMap, time::Instant};

use battery::BatteryWidgetData;
use clock::ClockState;
use now_playing::NowPlayingWidgetData;
use render::TextLayoutCache;
use veila_common::{FingerprintStatus, PowerAction, Secret};
use veila_renderer::avatar::AvatarAsset;
use weather::WeatherWidgetData;

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug)]
pub struct ShellState {
    mode: ShellMode,
    secret: Secret,
    submitted_secret_len: usize,
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
    static_scene_variant_cache: RefCell<HashMap<u32, String>>,
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

impl Clone for ShellState {
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            secret: self.secret.duplicate(),
            submitted_secret_len: self.submitted_secret_len,
            secret_selected: self.secret_selected,
            caps_lock_active: self.caps_lock_active,
            keyboard_layout_label: self.keyboard_layout_label.clone(),
            battery: self.battery.clone(),
            power_status_text: self.power_status_text.clone(),
            fingerprint_status: self.fingerprint_status,
            reveal_secret: self.reveal_secret,
            auth_revealed: self.auth_revealed,
            reveal_toggle_hovered: self.reveal_toggle_hovered,
            reveal_toggle_pressed: self.reveal_toggle_pressed,
            power_button_hovered: self.power_button_hovered,
            power_button_pressed: self.power_button_pressed,
            power_confirmation: self.power_confirmation,
            requested_power_action: self.requested_power_action,
            static_scene_revision: self.static_scene_revision,
            static_scene_variant_cache: RefCell::new(HashMap::new()),
            focused: self.focused,
            status: self.status.clone(),
            clock: self.clock.clone(),
            theme: self.theme.clone(),
            hint_text: self.hint_text.clone(),
            reveal_hint_text: self.reveal_hint_text.clone(),
            username_text: self.username_text.clone(),
            weather: self.weather.clone(),
            now_playing: self.now_playing.clone(),
            now_playing_transition: self.now_playing_transition.clone(),
            avatar: self.avatar.clone(),
            preview_grid_enabled: self.preview_grid_enabled,
            text_layout_cache: self.text_layout_cache.clone(),
            render_scale: self.render_scale,
        }
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new(ShellTheme::default(), None, None, true)
    }
}
