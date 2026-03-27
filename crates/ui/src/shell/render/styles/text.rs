use veila_renderer::text::{TextStyle, bundled_clock_font_family, resolve_font_family};

use super::{
    super::{ShellState, layout::SceneMetrics},
    color::{clock_scale, header_color, scaled_alpha, secondary_text_color, username_color},
};

const MAX_HEADER_TEXT_SCALE: u32 = 24;
const MAX_CLOCK_MERIDIEM_SCALE: u32 = 8;
const MAX_WEATHER_TEMPERATURE_SCALE: u32 = 24;
const MAX_WEATHER_LOCATION_SCALE: u32 = 12;
const DEFAULT_CLOCK_FONT_FAMILY: &str = "Geom";
const MAX_NOW_PLAYING_TITLE_SCALE: u32 = 4;
const MAX_NOW_PLAYING_ARTIST_SCALE: u32 = 3;

impl ShellState {
    pub(crate) fn keyboard_layout_text_style(&self) -> TextStyle {
        TextStyle::new(
            secondary_text_color(
                self.theme.keyboard_color.unwrap_or(self.theme.foreground),
                self.theme.keyboard_opacity,
                228,
            ),
            self.theme.keyboard_size.unwrap_or(2).clamp(1, 6),
        )
        .with_font_weight(600)
        .with_line_spacing(0)
    }

    pub(crate) fn clock_text_style(&self, metrics: SceneMetrics) -> TextStyle {
        let style = TextStyle::new(
            header_color(
                self.theme.clock_color.unwrap_or(self.theme.foreground),
                self.theme.clock_opacity,
                246,
            ),
            self.theme
                .clock_size
                .unwrap_or_else(|| clock_scale(metrics.avatar_size))
                .clamp(1, MAX_HEADER_TEXT_SCALE),
        )
        .with_line_spacing(0);

        let family = self
            .theme
            .clock_font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(bundled_clock_font_family)
            .or_else(|| self.theme.clock_font_family.clone())
            .unwrap_or_else(|| String::from(DEFAULT_CLOCK_FONT_FAMILY));

        let style = style.with_font_family(&family);
        match self.theme.clock_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style,
        }
    }

    pub(crate) fn clock_meridiem_text_style(&self, metrics: SceneMetrics) -> TextStyle {
        let clock_scale = self
            .theme
            .clock_size
            .unwrap_or_else(|| clock_scale(metrics.avatar_size))
            .clamp(1, MAX_HEADER_TEXT_SCALE);
        let meridiem_scale = self
            .theme
            .clock_meridiem_size
            .unwrap_or_else(|| clock_scale.div_ceil(3))
            .clamp(1, MAX_CLOCK_MERIDIEM_SCALE);
        let style = TextStyle::new(
            header_color(
                self.theme.clock_color.unwrap_or(self.theme.foreground),
                self.theme.clock_opacity,
                246,
            ),
            meridiem_scale,
        )
        .with_line_spacing(0);

        let family = self
            .theme
            .clock_font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(bundled_clock_font_family)
            .or_else(|| self.theme.clock_font_family.clone())
            .unwrap_or_else(|| String::from(DEFAULT_CLOCK_FONT_FAMILY));

        let style = style.with_font_family(&family);
        match self.theme.clock_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style,
        }
    }

    pub(crate) fn date_text_style(&self) -> TextStyle {
        let style = TextStyle::new(
            header_color(
                self.theme.date_color.unwrap_or(self.theme.foreground),
                self.theme.date_opacity,
                188,
            ),
            self.theme
                .date_size
                .unwrap_or(2)
                .clamp(1, MAX_HEADER_TEXT_SCALE),
        )
        .with_line_spacing(0);

        let style = match self
            .theme
            .date_font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(|| self.theme.date_font_family.clone())
        {
            Some(family) => style.with_font_family(&family),
            None => style,
        };

        match self.theme.date_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style,
        }
    }

    pub(crate) fn username_text_style(&self) -> TextStyle {
        TextStyle::new(
            username_color(
                self.theme.username_color.unwrap_or(self.theme.foreground),
                self.theme.username_opacity,
            ),
            self.theme.username_size.unwrap_or(2).clamp(1, 6),
        )
    }

    pub(crate) fn placeholder_text_style(&self) -> TextStyle {
        TextStyle::new(
            secondary_text_color(
                self.theme.placeholder_color.unwrap_or(self.theme.muted),
                self.theme.placeholder_opacity,
                154,
            ),
            2,
        )
    }

    pub(crate) fn status_text_style(&self) -> TextStyle {
        TextStyle::new(
            secondary_text_color(
                self.theme.status_color.unwrap_or(self.accent_color()),
                self.theme.status_opacity,
                255,
            ),
            2,
        )
    }

    pub(crate) fn caps_lock_text_style(&self) -> TextStyle {
        TextStyle::new(
            secondary_text_color(
                self.theme.status_color.unwrap_or(self.accent_color()),
                self.theme.status_opacity,
                214,
            ),
            1,
        )
        .with_line_spacing(0)
    }

    pub(crate) fn weather_temperature_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .weather_temperature_color
            .unwrap_or(self.theme.foreground);
        let style = TextStyle::new(
            base_color.with_alpha(scaled_alpha(
                base_color.alpha.min(232),
                self.theme.weather_temperature_opacity,
            )),
            self.theme
                .weather_temperature_size
                .or(self.theme.weather_size)
                .unwrap_or(2)
                .clamp(1, MAX_WEATHER_TEMPERATURE_SCALE),
        );

        let family = self
            .theme
            .weather_temperature_font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(|| self.theme.weather_temperature_font_family.clone());

        let style = match family {
            Some(family) => style.with_font_family(&family),
            None => style,
        };
        let style = match self.theme.weather_temperature_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style,
        };
        let style = match self.theme.weather_temperature_letter_spacing {
            Some(letter_spacing) => style.with_letter_spacing(letter_spacing),
            None => style,
        };

        style.with_line_spacing(0)
    }

    pub(crate) fn weather_location_text_style(&self) -> TextStyle {
        let temperature_scale = self
            .theme
            .weather_temperature_size
            .or(self.theme.weather_size)
            .unwrap_or(2)
            .clamp(1, 6);
        let location_scale = self
            .theme
            .weather_location_size
            .unwrap_or_else(|| temperature_scale.saturating_sub(1).max(1))
            .clamp(1, MAX_WEATHER_LOCATION_SCALE);
        let base_color = self
            .theme
            .weather_location_color
            .unwrap_or(self.theme.muted);

        TextStyle::new(
            base_color.with_alpha(scaled_alpha(
                base_color.alpha.min(184),
                self.theme.weather_location_opacity,
            )),
            location_scale,
        )
        .with_line_spacing(0)
    }

    pub(crate) fn now_playing_title_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .now_playing_title_color
            .unwrap_or(self.theme.foreground);
        let style = TextStyle::new(
            base_color.with_alpha(scaled_alpha(
                base_color.alpha.min(236),
                self.theme.now_playing_title_opacity,
            )),
            self.theme
                .now_playing_title_size
                .unwrap_or(2)
                .clamp(1, MAX_NOW_PLAYING_TITLE_SCALE),
        );
        let style = match self.theme.now_playing_title_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style.with_font_weight(600),
        };
        let family = self
            .theme
            .now_playing_title_font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(|| self.theme.now_playing_title_font_family.clone());

        match family {
            Some(family) => style.with_font_family(&family),
            None => style,
        }
        .with_line_spacing(0)
    }

    pub(crate) fn now_playing_artist_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .now_playing_artist_color
            .unwrap_or(self.theme.muted);
        let style = TextStyle::new(
            base_color.with_alpha(scaled_alpha(
                base_color.alpha.min(184),
                self.theme.now_playing_artist_opacity,
            )),
            self.theme
                .now_playing_artist_size
                .unwrap_or(1)
                .clamp(1, MAX_NOW_PLAYING_ARTIST_SCALE),
        );
        let style = match self.theme.now_playing_artist_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style,
        };
        let family = self
            .theme
            .now_playing_artist_font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(|| self.theme.now_playing_artist_font_family.clone());

        match family {
            Some(family) => style.with_font_family(&family),
            None => style,
        }
        .with_line_spacing(0)
    }
}
