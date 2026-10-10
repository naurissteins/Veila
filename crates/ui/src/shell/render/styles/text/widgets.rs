use super::super::color::secondary_text_color;
use super::{RenderContext, TextStyle};

const MAX_WEATHER_TEMPERATURE_FONT_SIZE_PX: u32 = 512;
const MAX_WEATHER_LOCATION_FONT_SIZE_PX: u32 = 512;
const DEFAULT_KEYBOARD_FONT_FAMILY: &str = "Geom";
const MAX_NOW_PLAYING_TITLE_FONT_SIZE_PX: u32 = 512;
const MAX_NOW_PLAYING_ARTIST_FONT_SIZE_PX: u32 = 512;
const MAX_KEYBOARD_FONT_SIZE_PX: u32 = 512;

impl RenderContext<'_> {
    pub(crate) fn keyboard_layout_text_style(&self) -> TextStyle {
        let style = TextStyle::new_px(
            secondary_text_color(
                self.theme.keyboard_color.unwrap_or(self.theme.foreground),
                None,
                228,
            ),
            self.theme
                .keyboard_size
                .unwrap_or(16)
                .clamp(1, MAX_KEYBOARD_FONT_SIZE_PX),
        )
        .with_font_weight(600)
        .with_line_spacing(0);

        self.apply_font_overrides(
            style,
            self.resolved_font_family(Some(DEFAULT_KEYBOARD_FONT_FAMILY)),
            Some(600),
            None,
        )
    }

    pub(crate) fn weather_temperature_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .weather
            .temperature_color
            .unwrap_or(self.theme.foreground);
        let style = TextStyle::new_px(
            base_color,
            self.theme
                .weather
                .temperature_font_size
                .unwrap_or(40)
                .clamp(1, MAX_WEATHER_TEMPERATURE_FONT_SIZE_PX),
        );

        let style = self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.weather.temperature_font_family.as_deref()),
            self.theme.weather.temperature_font_weight,
            self.theme.weather.temperature_font_style,
        );
        let style = match self.theme.weather.temperature_letter_spacing {
            Some(letter_spacing) => style.with_letter_spacing(letter_spacing),
            None => style,
        };

        style.with_line_spacing(0)
    }

    pub(crate) fn weather_location_text_style(&self) -> TextStyle {
        let location_font_size = self
            .theme
            .weather
            .location_font_size
            .unwrap_or(22)
            .clamp(1, MAX_WEATHER_LOCATION_FONT_SIZE_PX);
        let base_color = self
            .theme
            .weather
            .location_color
            .unwrap_or(self.theme.muted);
        let style = TextStyle::new_px(base_color, location_font_size).with_line_spacing(0);

        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.weather.location_font_family.as_deref()),
            self.theme.weather.location_font_weight,
            self.theme.weather.location_font_style,
        )
    }

    pub(crate) fn now_playing_title_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .now_playing
            .title_color
            .unwrap_or(self.theme.foreground);
        let style = TextStyle::new_px(
            base_color,
            self.theme
                .now_playing
                .title_font_size
                .unwrap_or(16)
                .clamp(1, MAX_NOW_PLAYING_TITLE_FONT_SIZE_PX),
        );
        let style = match self.theme.now_playing.title_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style.with_font_weight(600),
        };
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.now_playing.title_font_family.as_deref()),
            None,
            self.theme.now_playing.title_font_style,
        )
        .with_line_spacing(0)
    }

    pub(crate) fn now_playing_artist_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .now_playing
            .artist_color
            .unwrap_or(self.theme.muted);
        let style = TextStyle::new_px(
            base_color,
            self.theme
                .now_playing
                .artist_font_size
                .unwrap_or(16)
                .clamp(1, MAX_NOW_PLAYING_ARTIST_FONT_SIZE_PX),
        );
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.now_playing.artist_font_family.as_deref()),
            self.theme.now_playing.artist_font_weight,
            self.theme.now_playing.artist_font_style,
        )
        .with_line_spacing(0)
    }
}
