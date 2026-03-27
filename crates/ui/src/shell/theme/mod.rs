mod color;
#[cfg(test)]
mod tests;

use veila_common::{AppConfig, ClockFormat, WeatherAlignment};
use veila_renderer::ClearColor;

use self::color::{to_color, to_color_with_opacity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTheme {
    pub background: ClearColor,
    pub avatar_background: ClearColor,
    pub input: ClearColor,
    pub input_border: ClearColor,
    pub input_font_family: Option<String>,
    pub input_font_weight: Option<u16>,
    pub input_font_size: Option<u32>,
    pub input_width: Option<i32>,
    pub input_height: Option<i32>,
    pub input_radius: i32,
    pub input_border_width: Option<i32>,
    pub avatar_size: Option<i32>,
    pub avatar_placeholder_padding: Option<i32>,
    pub avatar_icon_color: Option<ClearColor>,
    pub avatar_ring_color: Option<ClearColor>,
    pub avatar_ring_width: Option<i32>,
    pub avatar_background_opacity: Option<u8>,
    pub username_font_family: Option<String>,
    pub username_font_weight: Option<u16>,
    pub username_color: Option<ClearColor>,
    pub username_opacity: Option<u8>,
    pub username_size: Option<u32>,
    pub avatar_gap: Option<i32>,
    pub username_gap: Option<i32>,
    pub status_gap: Option<i32>,
    pub clock_gap: Option<i32>,
    pub auth_stack_offset: Option<i32>,
    pub header_top_offset: Option<i32>,
    pub clock_font_family: Option<String>,
    pub clock_font_weight: Option<u16>,
    pub clock_format: ClockFormat,
    pub clock_meridiem_size: Option<u32>,
    pub clock_meridiem_offset_x: Option<i32>,
    pub clock_meridiem_offset_y: Option<i32>,
    pub clock_color: Option<ClearColor>,
    pub clock_opacity: Option<u8>,
    pub date_font_family: Option<String>,
    pub date_font_weight: Option<u16>,
    pub date_color: Option<ClearColor>,
    pub date_opacity: Option<u8>,
    pub clock_size: Option<u32>,
    pub date_size: Option<u32>,
    pub placeholder_color: Option<ClearColor>,
    pub placeholder_opacity: Option<u8>,
    pub eye_icon_color: Option<ClearColor>,
    pub eye_icon_opacity: Option<u8>,
    pub keyboard_background_color: ClearColor,
    pub keyboard_background_size: Option<i32>,
    pub keyboard_color: Option<ClearColor>,
    pub keyboard_opacity: Option<u8>,
    pub keyboard_size: Option<u32>,
    pub keyboard_top_offset: Option<i32>,
    pub keyboard_right_offset: Option<i32>,
    pub weather_size: Option<u32>,
    pub weather_opacity: Option<u8>,
    pub weather_icon_opacity: Option<u8>,
    pub weather_temperature_opacity: Option<u8>,
    pub weather_location_opacity: Option<u8>,
    pub weather_temperature_color: Option<ClearColor>,
    pub weather_location_color: Option<ClearColor>,
    pub weather_temperature_font_family: Option<String>,
    pub weather_temperature_font_weight: Option<u16>,
    pub weather_temperature_letter_spacing: Option<u32>,
    pub weather_location_font_family: Option<String>,
    pub weather_location_font_weight: Option<u16>,
    pub weather_temperature_size: Option<u32>,
    pub weather_location_size: Option<u32>,
    pub weather_icon_size: Option<i32>,
    pub weather_icon_gap: Option<i32>,
    pub weather_location_gap: Option<i32>,
    pub weather_left_offset: Option<i32>,
    pub weather_bottom_offset: Option<i32>,
    pub weather_horizontal_padding: Option<i32>,
    pub weather_bottom_padding: Option<i32>,
    pub weather_alignment: WeatherAlignment,
    pub now_playing_title_color: Option<ClearColor>,
    pub now_playing_artist_color: Option<ClearColor>,
    pub now_playing_fade_duration_ms: Option<u64>,
    pub now_playing_title_font_family: Option<String>,
    pub now_playing_artist_font_family: Option<String>,
    pub now_playing_title_font_weight: Option<u16>,
    pub now_playing_artist_font_weight: Option<u16>,
    pub now_playing_opacity: Option<u8>,
    pub now_playing_title_opacity: Option<u8>,
    pub now_playing_artist_opacity: Option<u8>,
    pub now_playing_artwork_opacity: Option<u8>,
    pub now_playing_title_size: Option<u32>,
    pub now_playing_artist_size: Option<u32>,
    pub now_playing_width: Option<i32>,
    pub now_playing_content_gap: Option<i32>,
    pub now_playing_text_gap: Option<i32>,
    pub now_playing_artwork_size: Option<i32>,
    pub now_playing_artwork_radius: Option<i32>,
    pub now_playing_right_padding: Option<i32>,
    pub now_playing_bottom_padding: Option<i32>,
    pub now_playing_right_offset: Option<i32>,
    pub now_playing_bottom_offset: Option<i32>,
    pub status_color: Option<ClearColor>,
    pub status_opacity: Option<u8>,
    pub input_mask_color: Option<ClearColor>,
    pub foreground: ClearColor,
    pub muted: ClearColor,
    pub pending: ClearColor,
    pub rejected: ClearColor,
}

impl Default for ShellTheme {
    fn default() -> Self {
        Self::from_config(&AppConfig::default())
    }
}

impl ShellTheme {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            background: to_color(config.background.color),
            avatar_background: config
                .visuals
                .avatar_background_color()
                .map(to_color)
                .unwrap_or_else(|| to_color(config.visuals.panel)),
            input: to_color_with_opacity(
                config.visuals.input_background_color(),
                config.visuals.input_background_opacity(),
            ),
            input_border: to_color_with_opacity(
                config.visuals.input_border_color(),
                config.visuals.input_border_opacity(),
            ),
            input_font_family: config.visuals.input_font_family().map(str::to_owned),
            input_font_weight: config.visuals.input_font_weight(),
            input_font_size: config.visuals.input_font_size().map(u32::from),
            input_width: config.visuals.input_width().map(i32::from),
            input_height: config.visuals.input_height().map(i32::from),
            input_radius: i32::from(config.visuals.input_radius()),
            input_border_width: config.visuals.input_border_width().map(i32::from),
            avatar_size: config.visuals.avatar_size().map(i32::from),
            avatar_placeholder_padding: config.visuals.avatar_placeholder_padding().map(i32::from),
            avatar_icon_color: config.visuals.avatar_icon_color().map(to_color),
            avatar_ring_color: config.visuals.avatar_ring_color().map(to_color),
            avatar_ring_width: config.visuals.avatar_ring_width().map(i32::from),
            avatar_background_opacity: config.visuals.avatar_background_opacity(),
            username_font_family: config.visuals.username_font_family().map(str::to_owned),
            username_font_weight: config.visuals.username_font_weight(),
            username_color: config.visuals.username_color().map(to_color),
            username_opacity: config.visuals.username_opacity(),
            username_size: config.visuals.username_size().map(u32::from),
            avatar_gap: config.visuals.avatar_gap().map(i32::from),
            username_gap: config.visuals.username_gap().map(i32::from),
            status_gap: config.visuals.status_gap().map(i32::from),
            clock_gap: config.visuals.clock_gap().map(i32::from),
            auth_stack_offset: config.visuals.auth_stack_offset().map(i32::from),
            header_top_offset: config.visuals.header_top_offset().map(i32::from),
            clock_font_family: config.visuals.clock_font_family().map(str::to_owned),
            clock_font_weight: config.visuals.clock_font_weight(),
            clock_format: config.visuals.clock_format(),
            clock_meridiem_size: config.visuals.clock_meridiem_size().map(u32::from),
            clock_meridiem_offset_x: config.visuals.clock_meridiem_offset_x().map(i32::from),
            clock_meridiem_offset_y: config.visuals.clock_meridiem_offset_y().map(i32::from),
            clock_color: config.visuals.clock_color().map(to_color),
            clock_opacity: config.visuals.clock_opacity(),
            date_font_family: config.visuals.date_font_family().map(str::to_owned),
            date_font_weight: config.visuals.date_font_weight(),
            date_color: config.visuals.date_color().map(to_color),
            date_opacity: config.visuals.date_opacity(),
            clock_size: config.visuals.clock_size().map(u32::from),
            date_size: config.visuals.date_size().map(u32::from),
            placeholder_color: config.visuals.placeholder_color().map(to_color),
            placeholder_opacity: config.visuals.placeholder_opacity(),
            eye_icon_color: config.visuals.eye_icon_color().map(to_color),
            eye_icon_opacity: config.visuals.eye_icon_opacity(),
            keyboard_background_color: config
                .visuals
                .keyboard_background_color()
                .map(to_color)
                .unwrap_or_else(|| ClearColor::rgba(18, 22, 30, 82)),
            keyboard_background_size: config.visuals.keyboard_background_size().map(i32::from),
            keyboard_color: config.visuals.keyboard_color().map(to_color),
            keyboard_opacity: config.visuals.keyboard_opacity(),
            keyboard_size: config.visuals.keyboard_size().map(u32::from),
            keyboard_top_offset: config.visuals.keyboard_top_offset().map(i32::from),
            keyboard_right_offset: config.visuals.keyboard_right_offset().map(i32::from),
            weather_size: config.visuals.weather_size().map(u32::from),
            weather_opacity: config.visuals.weather_opacity(),
            weather_icon_opacity: config.visuals.weather_icon_opacity(),
            weather_temperature_opacity: config.visuals.weather_temperature_opacity(),
            weather_location_opacity: config.visuals.weather_location_opacity(),
            weather_temperature_color: config.visuals.weather_temperature_color().map(to_color),
            weather_location_color: config.visuals.weather_location_color().map(to_color),
            weather_temperature_font_family: config
                .visuals
                .weather_temperature_font_family()
                .map(str::to_owned),
            weather_temperature_font_weight: config.visuals.weather_temperature_font_weight(),
            weather_temperature_letter_spacing: config
                .visuals
                .weather_temperature_letter_spacing()
                .map(u32::from),
            weather_location_font_family: config
                .visuals
                .weather_location_font_family()
                .map(str::to_owned),
            weather_location_font_weight: config.visuals.weather_location_font_weight(),
            weather_temperature_size: config.visuals.weather_temperature_size().map(u32::from),
            weather_location_size: config.visuals.weather_location_size().map(u32::from),
            weather_icon_size: config.visuals.weather_icon_size().map(i32::from),
            weather_icon_gap: config.visuals.weather_icon_gap().map(i32::from),
            weather_location_gap: config.visuals.weather_location_gap().map(i32::from),
            weather_left_offset: config.visuals.weather_left_offset().map(i32::from),
            weather_bottom_offset: config.visuals.weather_bottom_offset().map(i32::from),
            weather_horizontal_padding: config.visuals.weather_horizontal_padding().map(i32::from),
            weather_bottom_padding: config.visuals.weather_bottom_padding().map(i32::from),
            weather_alignment: config.visuals.weather_alignment(),
            now_playing_title_color: config.visuals.now_playing_title_color().map(to_color),
            now_playing_artist_color: config.visuals.now_playing_artist_color().map(to_color),
            now_playing_fade_duration_ms: config
                .visuals
                .now_playing_fade_duration_ms()
                .map(u64::from),
            now_playing_title_font_family: config
                .visuals
                .now_playing_title_font_family()
                .map(str::to_owned),
            now_playing_artist_font_family: config
                .visuals
                .now_playing_artist_font_family()
                .map(str::to_owned),
            now_playing_title_font_weight: config.visuals.now_playing_title_font_weight(),
            now_playing_artist_font_weight: config.visuals.now_playing_artist_font_weight(),
            now_playing_opacity: config.visuals.now_playing_opacity(),
            now_playing_title_opacity: config.visuals.now_playing_title_opacity(),
            now_playing_artist_opacity: config.visuals.now_playing_artist_opacity(),
            now_playing_artwork_opacity: config.visuals.now_playing_artwork_opacity(),
            now_playing_title_size: config.visuals.now_playing_title_size().map(u32::from),
            now_playing_artist_size: config.visuals.now_playing_artist_size().map(u32::from),
            now_playing_width: config.visuals.now_playing_width().map(i32::from),
            now_playing_content_gap: config.visuals.now_playing_content_gap().map(i32::from),
            now_playing_text_gap: config.visuals.now_playing_text_gap().map(i32::from),
            now_playing_artwork_size: config.visuals.now_playing_artwork_size().map(i32::from),
            now_playing_artwork_radius: config.visuals.now_playing_artwork_radius().map(i32::from),
            now_playing_right_padding: config.visuals.now_playing_right_padding().map(i32::from),
            now_playing_bottom_padding: config.visuals.now_playing_bottom_padding().map(i32::from),
            now_playing_right_offset: config.visuals.now_playing_right_offset().map(i32::from),
            now_playing_bottom_offset: config.visuals.now_playing_bottom_offset().map(i32::from),
            status_color: config.visuals.status_color().map(to_color),
            status_opacity: config.visuals.status_opacity(),
            input_mask_color: config.visuals.input_mask_color().map(to_color),
            foreground: to_color(config.visuals.foreground_color()),
            muted: to_color(config.visuals.muted_color()),
            pending: to_color(config.visuals.pending_color()),
            rejected: to_color(config.visuals.rejected_color()),
        }
    }
}
