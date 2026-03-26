mod avatar;
mod clock;
mod input;
mod pointer;
mod render;
mod state;
#[cfg(test)]
mod tests;
mod theme;
mod weather;

pub use theme::ShellTheme;

use std::{cell::RefCell, time::Instant};

use clock::ClockState;
use render::TextLayoutCache;
use veila_renderer::avatar::AvatarAsset;
use weather::WeatherWidgetData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    None,
    Submit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKey {
    Character(char),
    Backspace,
    Enter,
    Escape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShellStatus {
    Idle,
    Pending,
    Rejected {
        retry_until: Option<Instant>,
        displayed_retry_seconds: Option<u64>,
    },
}

#[derive(Debug, Clone)]
pub struct ShellState {
    secret: String,
    caps_lock_active: bool,
    reveal_secret: bool,
    reveal_toggle_hovered: bool,
    reveal_toggle_pressed: bool,
    static_scene_revision: u64,
    focused: bool,
    status: ShellStatus,
    clock: ClockState,
    theme: ShellTheme,
    hint_text: String,
    username_text: Option<String>,
    weather: Option<WeatherWidgetData>,
    avatar: AvatarAsset,
    text_layout_cache: RefCell<TextLayoutCache>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new(ShellTheme::default(), None, None, true)
    }
}
