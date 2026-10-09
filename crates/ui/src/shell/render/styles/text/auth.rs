use crate::shell::ShellStatus;

use super::super::color::{secondary_text_color, username_color};
use super::{RenderContext, TextStyle};

const MAX_USERNAME_FONT_SIZE_PX: u32 = 512;
const MAX_INPUT_FONT_SIZE_PX: u32 = 512;
const MAX_REVEAL_FONT_SIZE_PX: u32 = 512;

impl RenderContext<'_> {
    pub(crate) fn username_text_style(&self) -> TextStyle {
        let style = TextStyle::new_px(
            username_color(self.theme.username.color.unwrap_or(self.theme.foreground)),
            self.theme
                .username
                .font_size
                .unwrap_or(28)
                .clamp(1, MAX_USERNAME_FONT_SIZE_PX),
        );
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.username.font_family.as_deref()),
            self.theme.username.font_weight,
            self.theme.username.font_style,
        )
    }

    pub(crate) fn placeholder_text_style(&self) -> TextStyle {
        let style = TextStyle::new_px(
            secondary_text_color(
                self.theme.placeholder.color.unwrap_or(self.theme.muted),
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
                .reveal
                .color
                .unwrap_or(self.theme.placeholder.color.unwrap_or(self.theme.muted)),
            None,
            154,
        );
        let style = match self.theme.reveal.font_size {
            Some(font_size) => {
                TextStyle::new_px(color, font_size.clamp(1, MAX_REVEAL_FONT_SIZE_PX))
            }
            None => TextStyle::new_px(color, self.input_font_size_px()),
        };
        // Reveal overrides inherit each missing input-font property independently.
        self.apply_font_overrides(
            style,
            self.resolved_font_family(self.theme.reveal.font_family.as_deref())
                .or_else(|| self.resolved_font_family(self.theme.input.font_family.as_deref())),
            self.theme
                .reveal
                .font_weight
                .or(self.theme.input.font_weight),
            self.theme.reveal.font_style.or(self.theme.input.font_style),
        )
    }

    pub(crate) fn revealed_secret_text_style(&self) -> TextStyle {
        self.apply_input_font(TextStyle::new_px(
            self.theme.foreground.with_alpha(236),
            self.input_font_size_px(),
        ))
    }

    pub(crate) fn status_text_style(&self) -> TextStyle {
        let color = match self.shell.status {
            ShellStatus::Pending { .. } => self
                .theme
                .status
                .pending_color
                .or(self.theme.status.color)
                .unwrap_or(self.theme.pending),
            ShellStatus::Rejected { .. } => self
                .theme
                .status
                .rejected_color
                .or(self.theme.status.color)
                .unwrap_or(self.theme.rejected),
            ShellStatus::Challenge { .. } | ShellStatus::Notice { .. } | ShellStatus::Idle => self
                .theme
                .status
                .color
                .unwrap_or(self.theme.input.border_color),
        };
        TextStyle::new(secondary_text_color(color, None, 224), 2)
    }

    pub(crate) fn input_status_text_style(&self) -> TextStyle {
        let color = match self.shell.status {
            ShellStatus::Pending { .. } => self
                .theme
                .status
                .pending_color
                .or(self.theme.status.color)
                .unwrap_or(self.theme.pending),
            ShellStatus::Rejected { .. } => self
                .theme
                .status
                .rejected_color
                .or(self.theme.status.color)
                .unwrap_or(self.theme.rejected),
            ShellStatus::Challenge { .. } | ShellStatus::Notice { .. } | ShellStatus::Idle => self
                .theme
                .status
                .color
                .unwrap_or(self.theme.input.border_color),
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
            self.resolved_font_family(self.theme.input.font_family.as_deref()),
            self.theme.input.font_weight,
            self.theme.input.font_style,
        )
    }

    fn input_font_size_px(&self) -> u32 {
        self.theme
            .input
            .font_size
            .unwrap_or(16)
            .clamp(1, MAX_INPUT_FONT_SIZE_PX)
    }
}
