use veila_renderer::text::{bundled_clock_font_family, resolve_font_family};

use super::super::color::header_color;
use super::{RenderContext, TextStyle};
use crate::shell::render::layout::SceneMetrics;

const MAX_CLOCK_FONT_SIZE_PX: u32 = 1024;
const MAX_DATE_FONT_SIZE_PX: u32 = 512;
const MAX_CLOCK_MERIDIEM_FONT_SIZE_PX: u32 = 512;
const DEFAULT_CLOCK_FONT_FAMILY: &str = "Geom";

impl RenderContext<'_> {
    pub(crate) fn clock_text_style(&self, _metrics: SceneMetrics) -> TextStyle {
        let style = TextStyle::new_px(
            header_color(
                self.theme.clock.color.unwrap_or(self.theme.foreground),
                None,
                246,
            ),
            self.theme
                .clock
                .font_size
                .unwrap_or(88)
                .clamp(1, MAX_CLOCK_FONT_SIZE_PX),
        )
        .with_line_spacing(0);

        let family = self
            .theme
            .clock
            .font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(bundled_clock_font_family)
            .or_else(|| self.theme.clock.font_family.clone())
            .unwrap_or_else(|| String::from(DEFAULT_CLOCK_FONT_FAMILY));

        self.apply_font_overrides(
            style,
            Some(family),
            self.theme.clock.font_weight,
            self.theme.clock.font_style,
        )
    }

    pub(crate) fn clock_meridiem_text_style(&self, _metrics: SceneMetrics) -> TextStyle {
        let clock_font_size = self
            .theme
            .clock
            .font_size
            .unwrap_or(88)
            .clamp(1, MAX_CLOCK_FONT_SIZE_PX);
        let meridiem_font_size = self
            .theme
            .clock
            .meridiem_font_size
            .unwrap_or_else(|| (clock_font_size / 4).max(1))
            .clamp(1, MAX_CLOCK_MERIDIEM_FONT_SIZE_PX);
        let style = TextStyle::new_px(
            header_color(
                self.theme.clock.color.unwrap_or(self.theme.foreground),
                None,
                246,
            ),
            meridiem_font_size,
        )
        .with_line_spacing(0);

        let family = self
            .theme
            .clock
            .font_family
            .as_deref()
            .and_then(resolve_font_family)
            .or_else(bundled_clock_font_family)
            .or_else(|| self.theme.clock.font_family.clone())
            .unwrap_or_else(|| String::from(DEFAULT_CLOCK_FONT_FAMILY));

        self.apply_font_overrides(
            style,
            Some(family),
            self.theme.clock.font_weight,
            self.theme.clock.font_style,
        )
    }

    pub(crate) fn date_text_style(&self) -> TextStyle {
        let style = TextStyle::new_px(
            header_color(
                self.theme.date.color.unwrap_or(self.theme.foreground),
                None,
                188,
            ),
            self.theme
                .date
                .font_size
                .unwrap_or(16)
                .clamp(1, MAX_DATE_FONT_SIZE_PX),
        )
        .with_line_spacing(0);

        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.date.font_family.as_deref()),
            self.theme.date.font_weight,
            self.theme.date.font_style,
        )
    }
}
