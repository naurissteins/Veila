use crate::shell::ShellStatus;
use veila_common::FontStyle as ConfigFontStyle;
use veila_renderer::text::{
    FontStyle as RendererFontStyle, TextStyle, bundled_clock_font_family, resolve_font_family,
};

use super::{
    super::{ShellState, layout::SceneMetrics},
    color::{header_color, secondary_text_color, username_color},
};

const MAX_CLOCK_FONT_SIZE_PX: u32 = 1024;
const MAX_DATE_FONT_SIZE_PX: u32 = 512;
const MAX_CLOCK_MERIDIEM_FONT_SIZE_PX: u32 = 512;
const MAX_WEATHER_TEMPERATURE_FONT_SIZE_PX: u32 = 512;
const MAX_WEATHER_LOCATION_FONT_SIZE_PX: u32 = 512;
const DEFAULT_CLOCK_FONT_FAMILY: &str = "Geom";
const DEFAULT_KEYBOARD_FONT_FAMILY: &str = "Geom";
const MAX_NOW_PLAYING_TITLE_FONT_SIZE_PX: u32 = 512;
const MAX_NOW_PLAYING_ARTIST_FONT_SIZE_PX: u32 = 512;
const MAX_USERNAME_FONT_SIZE_PX: u32 = 512;
const MAX_INPUT_FONT_SIZE_PX: u32 = 512;
const MAX_REVEAL_FONT_SIZE_PX: u32 = 512;
const MAX_CUSTOM_LAYER_FONT_SIZE_PX: u32 = 512;
const MAX_KEYBOARD_FONT_SIZE_PX: u32 = 512;

impl ShellState {
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

    pub(crate) fn clock_text_style(&self, _metrics: SceneMetrics) -> TextStyle {
        let style = TextStyle::new_px(
            header_color(
                self.theme.clock_color.unwrap_or(self.theme.foreground),
                None,
                246,
            ),
            self.theme
                .clock_font_size
                .unwrap_or(88)
                .clamp(1, MAX_CLOCK_FONT_SIZE_PX),
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

        self.apply_font_overrides(
            style,
            Some(family),
            self.theme.clock_font_weight,
            self.theme.clock_font_style,
        )
    }

    pub(crate) fn clock_meridiem_text_style(&self, _metrics: SceneMetrics) -> TextStyle {
        let clock_font_size = self
            .theme
            .clock_font_size
            .unwrap_or(88)
            .clamp(1, MAX_CLOCK_FONT_SIZE_PX);
        let meridiem_font_size = self
            .theme
            .clock_meridiem_font_size
            .unwrap_or_else(|| (clock_font_size / 4).max(1))
            .clamp(1, MAX_CLOCK_MERIDIEM_FONT_SIZE_PX);
        let style = TextStyle::new_px(
            header_color(
                self.theme.clock_color.unwrap_or(self.theme.foreground),
                None,
                246,
            ),
            meridiem_font_size,
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

        self.apply_font_overrides(
            style,
            Some(family),
            self.theme.clock_font_weight,
            self.theme.clock_font_style,
        )
    }

    pub(crate) fn date_text_style(&self) -> TextStyle {
        let style = TextStyle::new_px(
            header_color(
                self.theme.date_color.unwrap_or(self.theme.foreground),
                None,
                188,
            ),
            self.theme
                .date_font_size
                .unwrap_or(16)
                .clamp(1, MAX_DATE_FONT_SIZE_PX),
        )
        .with_line_spacing(0);

        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.date_font_family.as_deref()),
            self.theme.date_font_weight,
            self.theme.date_font_style,
        )
    }

    pub(crate) fn username_text_style(&self) -> TextStyle {
        let style = TextStyle::new_px(
            username_color(self.theme.username_color.unwrap_or(self.theme.foreground)),
            self.theme
                .username_font_size
                .unwrap_or(28)
                .clamp(1, MAX_USERNAME_FONT_SIZE_PX),
        );
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.username_font_family.as_deref()),
            self.theme.username_font_weight,
            self.theme.username_font_style,
        )
    }

    pub(crate) fn placeholder_text_style(&self) -> TextStyle {
        let style = TextStyle::new_px(
            secondary_text_color(
                self.theme.placeholder_color.unwrap_or(self.theme.muted),
                None,
                154,
            ),
            self.input_font_size_px(),
        );
        self.apply_input_font(style)
    }

    pub(crate) fn reveal_text_style(&self) -> TextStyle {
        let color = secondary_text_color(
            self.theme
                .reveal_color
                .unwrap_or(self.theme.placeholder_color.unwrap_or(self.theme.muted)),
            None,
            154,
        );
        let style = match self.theme.reveal_font_size {
            Some(font_size) => {
                TextStyle::new_px(color, font_size.clamp(1, MAX_REVEAL_FONT_SIZE_PX))
            }
            None => TextStyle::new_px(color, self.input_font_size_px()),
        };
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.reveal_font_family.as_deref())
                .or_else(|| self.resolved_font_family(self.theme.input_font_family.as_deref())),
            self.theme
                .reveal_font_weight
                .or(self.theme.input_font_weight),
            self.theme.reveal_font_style.or(self.theme.input_font_style),
        )
    }

    pub(crate) fn revealed_secret_text_style(&self) -> TextStyle {
        self.apply_input_font(TextStyle::new_px(
            self.theme.foreground.with_alpha(236),
            self.input_font_size_px(),
        ))
    }

    pub(crate) fn status_text_style(&self) -> TextStyle {
        let color = match self.status {
            ShellStatus::Pending { .. } => self
                .theme
                .status_pending_color
                .or(self.theme.status_color)
                .unwrap_or(self.theme.pending),
            ShellStatus::Rejected { .. } => self
                .theme
                .status_rejected_color
                .or(self.theme.status_color)
                .unwrap_or(self.theme.rejected),
            ShellStatus::Idle => self.theme.status_color.unwrap_or(self.theme.input_border),
        };
        TextStyle::new(secondary_text_color(color, None, 224), 2)
    }

    pub(crate) fn input_status_text_style(&self) -> TextStyle {
        let color = match self.status {
            ShellStatus::Pending { .. } => self
                .theme
                .status_pending_color
                .or(self.theme.status_color)
                .unwrap_or(self.theme.pending),
            ShellStatus::Rejected { .. } => self
                .theme
                .status_rejected_color
                .or(self.theme.status_color)
                .unwrap_or(self.theme.rejected),
            ShellStatus::Idle => self.theme.status_color.unwrap_or(self.theme.input_border),
        };
        self.apply_input_font(
            TextStyle::new_px(
                secondary_text_color(color, None, 224),
                self.input_font_size_px(),
            )
            .with_line_spacing(0),
        )
    }

    fn apply_input_font(&self, style: TextStyle) -> TextStyle {
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.input_font_family.as_deref()),
            self.theme.input_font_weight,
            self.theme.input_font_style,
        )
    }

    fn input_font_size_px(&self) -> u32 {
        self.theme
            .input_font_size
            .unwrap_or(16)
            .clamp(1, MAX_INPUT_FONT_SIZE_PX)
    }

    pub(crate) fn weather_temperature_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .weather_temperature_color
            .unwrap_or(self.theme.foreground);
        let style = TextStyle::new_px(
            base_color,
            self.theme
                .weather_temperature_font_size
                .unwrap_or(40)
                .clamp(1, MAX_WEATHER_TEMPERATURE_FONT_SIZE_PX),
        );

        let style = self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.weather_temperature_font_family.as_deref()),
            self.theme.weather_temperature_font_weight,
            self.theme.weather_temperature_font_style,
        );
        let style = match self.theme.weather_temperature_letter_spacing {
            Some(letter_spacing) => style.with_letter_spacing(letter_spacing),
            None => style,
        };

        style.with_line_spacing(0)
    }

    pub(crate) fn weather_location_text_style(&self) -> TextStyle {
        let location_font_size = self
            .theme
            .weather_location_font_size
            .unwrap_or(22)
            .clamp(1, MAX_WEATHER_LOCATION_FONT_SIZE_PX);
        let base_color = self
            .theme
            .weather_location_color
            .unwrap_or(self.theme.muted);
        let style = TextStyle::new_px(base_color, location_font_size).with_line_spacing(0);

        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.weather_location_font_family.as_deref()),
            self.theme.weather_location_font_weight,
            self.theme.weather_location_font_style,
        )
    }

    pub(crate) fn now_playing_title_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .now_playing_title_color
            .unwrap_or(self.theme.foreground);
        let style = TextStyle::new_px(
            if base_color.alpha == u8::MAX {
                base_color.with_alpha(175)
            } else {
                base_color
            },
            self.theme
                .now_playing_title_font_size
                .unwrap_or(16)
                .clamp(1, MAX_NOW_PLAYING_TITLE_FONT_SIZE_PX),
        );
        let style = match self.theme.now_playing_title_font_weight {
            Some(weight) => style.with_font_weight(weight),
            None => style.with_font_weight(600),
        };
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.now_playing_title_font_family.as_deref()),
            None,
            self.theme.now_playing_title_font_style,
        )
        .with_line_spacing(0)
    }

    pub(crate) fn now_playing_artist_text_style(&self) -> TextStyle {
        let base_color = self
            .theme
            .now_playing_artist_color
            .unwrap_or(self.theme.muted);
        let style = TextStyle::new_px(
            if base_color.alpha == u8::MAX {
                base_color.with_alpha(99)
            } else {
                base_color
            },
            self.theme
                .now_playing_artist_font_size
                .unwrap_or(16)
                .clamp(1, MAX_NOW_PLAYING_ARTIST_FONT_SIZE_PX),
        );
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.now_playing_artist_font_family.as_deref()),
            self.theme.now_playing_artist_font_weight,
            self.theme.now_playing_artist_font_style,
        )
        .with_line_spacing(0)
    }

    pub(crate) fn custom_layer_text_style(
        &self,
        layer: &crate::shell::theme::VisualLayer,
    ) -> TextStyle {
        let style = TextStyle::new_px(
            layer.color,
            layer.font_size.clamp(1, MAX_CUSTOM_LAYER_FONT_SIZE_PX),
        )
        .with_line_spacing(0);

        self.apply_font_overrides(
            style,
            self.resolved_font_family(layer.font_family.as_deref()),
            layer.font_weight,
            layer.font_style,
        )
    }

    fn resolved_font_family(&self, family: Option<&str>) -> Option<String> {
        family
            .and_then(resolve_font_family)
            .or_else(|| family.map(str::to_owned))
    }

    fn apply_font_overrides(
        &self,
        style: TextStyle,
        family: Option<String>,
        weight: Option<u16>,
        font_style: Option<ConfigFontStyle>,
    ) -> TextStyle {
        let style = match family {
            Some(family) => style.with_font_family(&family),
            None => style,
        };
        let style = match weight {
            Some(weight) => style.with_font_weight(weight),
            None => style,
        };

        match font_style {
            Some(ConfigFontStyle::Normal) => style.with_font_style(RendererFontStyle::Normal),
            Some(ConfigFontStyle::Italic) => style.with_font_style(RendererFontStyle::Italic),
            None => style,
        }
    }
}
