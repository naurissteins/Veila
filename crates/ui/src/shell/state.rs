use std::{cell::RefCell, path::PathBuf};

use veila_common::{BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot, WeatherUnit};
use veila_renderer::ClearColor;

use super::{
    ClockState, NowPlayingTransition, ShellState, ShellStatus, ShellTheme, TextLayoutCache,
    avatar::{load_avatar, username_text},
    battery::widget_data as battery_widget_data,
    now_playing::{same_widget_data, widget_data as now_playing_widget_data},
    weather::widget_data,
};

impl ShellState {
    pub fn layer_cache_variant(&self) -> Option<String> {
        if !self.theme.layer_enabled {
            return None;
        }

        let color = self.theme.layer_color;
        let border = self
            .theme
            .layer_border_color
            .unwrap_or(ClearColor::rgba(0, 0, 0, 0));
        Some(format!(
            "layer:v3:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}",
            self.theme.layer_style,
            self.theme.layer_mode,
            self.theme.layer_alignment,
            self.theme.layer_full_width,
            self.theme.layer_width,
            self.theme.layer_offset_x,
            self.theme.layer_left_padding,
            self.theme.layer_right_padding,
            self.theme.layer_top_padding,
            self.theme.layer_bottom_padding,
            self.theme.layer_radius,
            color.red,
            color.green,
            color.blue,
            color.alpha,
            self.theme.layer_blur_radius,
            border.red,
            border.green,
            border.blue,
            border.alpha,
            self.theme.layer_border_width,
        ))
    }

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
        battery_snapshot: Option<BatterySnapshot>,
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
            battery_snapshot,
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
        battery_snapshot: Option<BatterySnapshot>,
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
            battery_snapshot,
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
        battery_snapshot: Option<BatterySnapshot>,
        now_playing_snapshot: Option<NowPlayingSnapshot>,
    ) -> Self {
        Self {
            secret: String::new(),
            caps_lock_active: false,
            keyboard_layout_label: None,
            battery: battery_widget_data(battery_snapshot),
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
            now_playing_transition: None,
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

    pub fn set_now_playing_snapshot(&mut self, snapshot: Option<NowPlayingSnapshot>) {
        let next = now_playing_widget_data(snapshot);
        if same_widget_data(self.now_playing.as_ref(), next.as_ref()) {
            return;
        }

        self.now_playing_transition = Some(NowPlayingTransition {
            previous: self.now_playing.clone(),
            started_at: std::time::Instant::now(),
        });
        self.now_playing = next;
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
        battery_snapshot: Option<BatterySnapshot>,
        now_playing_snapshot: Option<NowPlayingSnapshot>,
    ) {
        self.theme = theme;
        self.clock = ClockState::current(self.theme.clock_format);
        self.hint_text = user_hint
            .filter(|hint| !hint.trim().is_empty())
            .unwrap_or_else(|| String::from("Type your password to unlock"));
        if !self.theme.eye_enabled {
            self.reveal_secret = false;
            self.reveal_toggle_hovered = false;
            self.reveal_toggle_pressed = false;
        }
        self.username_text = username_text(show_username, username_override);
        self.weather = widget_data(weather_location, weather_snapshot, weather_unit);
        self.battery = battery_widget_data(battery_snapshot);
        self.now_playing = now_playing_widget_data(now_playing_snapshot);
        self.now_playing_transition = None;
        self.avatar = load_avatar(avatar_path);
        self.bump_static_scene_revision();
    }

    pub fn static_scene_revision(&self) -> u64 {
        self.static_scene_revision
    }
}
