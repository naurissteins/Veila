use veila_renderer::text::{TextStyle, bundled_clock_font_family, resolve_font_family};

use super::{
    super::{ShellState, layout::SceneMetrics},
    color::{clock_scale, header_color, secondary_text_color, username_color},
};

const MAX_HEADER_TEXT_SCALE: u32 = 24;
const DEFAULT_CLOCK_FONT_FAMILY: &str = "Prototype";

impl ShellState {
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

        style.with_font_family(&family)
    }

    pub(crate) fn date_text_style(&self) -> TextStyle {
        TextStyle::new(
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
        .with_line_spacing(0)
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
}
