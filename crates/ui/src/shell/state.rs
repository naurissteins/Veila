use std::{cell::RefCell, path::PathBuf};

use veila_common::{NowPlayingSnapshot, WeatherSnapshot, WeatherUnit};

use super::{
    ClockState, ShellState, ShellStatus, ShellTheme, TextLayoutCache,
    avatar::{load_avatar, username_text},
    now_playing::widget_data as now_playing_widget_data,
    weather::widget_data,
};

impl ShellState {
    pub fn new(
        theme: ShellTheme,
        user_hint: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
    ) -> Self {
        Self::new_with_weather(
            theme,
            user_hint,
            None,
            avatar_path,
            show_username,
            None,
            None,
            WeatherUnit::default(),
            None,
        )
    }

    pub fn new_with_username(
        theme: ShellTheme,
        user_hint: Option<String>,
        username_override: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
    ) -> Self {
        Self::new_with_weather(
            theme,
            user_hint,
            username_override,
            avatar_path,
            show_username,
            None,
            None,
            WeatherUnit::default(),
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_username_and_weather(
        theme: ShellTheme,
        user_hint: Option<String>,
        username_override: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
        weather_location: Option<String>,
        weather_snapshot: Option<WeatherSnapshot>,
        weather_unit: WeatherUnit,
    ) -> Self {
        Self::new_with_username_and_widgets(
            theme,
            user_hint,
            username_override,
            avatar_path,
            show_username,
            weather_location,
            weather_snapshot,
            weather_unit,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_username_and_widgets(
        theme: ShellTheme,
        user_hint: Option<String>,
        username_override: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
        weather_location: Option<String>,
        weather_snapshot: Option<WeatherSnapshot>,
        weather_unit: WeatherUnit,
        now_playing_snapshot: Option<NowPlayingSnapshot>,
    ) -> Self {
        Self::new_with_weather(
            theme,
            user_hint,
            username_override,
            avatar_path,
            show_username,
            weather_location,
            weather_snapshot,
            weather_unit,
            now_playing_snapshot,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_weather(
        theme: ShellTheme,
        user_hint: Option<String>,
        username_override: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
        weather_location: Option<String>,
        weather_snapshot: Option<WeatherSnapshot>,
        weather_unit: WeatherUnit,
        now_playing_snapshot: Option<NowPlayingSnapshot>,
    ) -> Self {
        Self {
            secret: String::new(),
            caps_lock_active: false,
            keyboard_layout_label: None,
            reveal_secret: false,
            reveal_toggle_hovered: false,
            reveal_toggle_pressed: false,
            static_scene_revision: 1,
            focused: true,
            status: ShellStatus::Idle,
            clock: ClockState::current(theme.clock_format),
            theme,
            hint_text: user_hint
                .filter(|hint| !hint.trim().is_empty())
                .unwrap_or_else(|| String::from("Type your password to unlock")),
            username_text: username_text(show_username, username_override),
            weather: widget_data(weather_location, weather_snapshot, weather_unit),
            now_playing: now_playing_widget_data(now_playing_snapshot),
            avatar: load_avatar(avatar_path),
            text_layout_cache: RefCell::new(TextLayoutCache::default()),
        }
    }

    pub fn set_focus(&mut self, focused: bool) {
        if self.focused != focused {
            self.bump_static_scene_revision();
        }
        self.focused = focused;
    }

    pub fn set_caps_lock_active(&mut self, active: bool) -> bool {
        if self.caps_lock_active == active {
            return false;
        }

        self.caps_lock_active = active;
        true
    }

    pub fn set_keyboard_layout_label(&mut self, label: Option<String>) -> bool {
        if self.keyboard_layout_label == label {
            return false;
        }

        self.keyboard_layout_label = label;
        true
    }

    pub fn apply_theme(
        &mut self,
        theme: ShellTheme,
        user_hint: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
    ) {
        self.apply_theme_with_username(theme, user_hint, None, avatar_path, show_username);
    }

    pub fn apply_theme_with_username(
        &mut self,
        theme: ShellTheme,
        user_hint: Option<String>,
        username_override: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
    ) {
        self.apply_theme_with_username_and_weather(
            theme,
            user_hint,
            username_override,
            avatar_path,
            show_username,
            None,
            None,
            WeatherUnit::default(),
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_theme_with_username_and_weather(
        &mut self,
        theme: ShellTheme,
        user_hint: Option<String>,
        username_override: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
        weather_location: Option<String>,
        weather_snapshot: Option<WeatherSnapshot>,
        weather_unit: WeatherUnit,
        now_playing_snapshot: Option<NowPlayingSnapshot>,
    ) {
        self.theme = theme;
        self.clock = ClockState::current(self.theme.clock_format);
        self.hint_text = user_hint
            .filter(|hint| !hint.trim().is_empty())
            .unwrap_or_else(|| String::from("Type your password to unlock"));
        self.username_text = username_text(show_username, username_override);
        self.weather = widget_data(weather_location, weather_snapshot, weather_unit);
        self.now_playing = now_playing_widget_data(now_playing_snapshot);
        self.avatar = load_avatar(avatar_path);
        self.bump_static_scene_revision();
    }

    pub fn static_scene_revision(&self) -> u64 {
        self.static_scene_revision
    }
}
