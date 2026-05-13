use crate::shell::ShellStatus;
use veila_renderer::{
    avatar::AvatarStyle,
    icon::IconStyle,
    masked::MaskedInputStyle,
    shape::{BorderStyle, PillStyle},
};

use super::{
    super::ShellState,
    color::{avatar_ring_color, eye_icon_alpha, percent_to_alpha, styled_alpha},
};

impl ShellState {
    fn render_scale_i32(&self) -> i32 {
        self.render_scale.max(1) as i32
    }

    fn scaled_px(&self, value: i32) -> i32 {
        value.saturating_mul(self.render_scale_i32())
    }

    pub(crate) fn input_style(&self) -> PillStyle {
        let selection_active = self.secret_selected;
        let base_border = if matches!(self.status, ShellStatus::Rejected { .. }) {
            self.theme
                .status_rejected_color
                .or(self.theme.status_color)
                .unwrap_or(self.theme.rejected)
        } else {
            self.theme.input_border
        };
        let border = if selection_active {
            base_border.with_alpha(if base_border.alpha == u8::MAX {
                248
            } else {
                base_border.alpha.max(148)
            })
        } else if self.focused {
            base_border.with_alpha(styled_alpha(base_border.alpha, 240))
        } else {
            base_border.with_alpha(styled_alpha(base_border.alpha, 210))
        };
        let border_width = self.theme.input_border_width.unwrap_or(2).max(0);

        let style = PillStyle::new(self.theme.input.with_alpha(if selection_active {
            if self.theme.input.alpha == u8::MAX {
                244
            } else {
                self.theme.input.alpha.max(88)
            }
        } else {
            styled_alpha(self.theme.input.alpha, 232)
        }))
        .with_radius(self.theme.input_radius);

        if border_width == 0 {
            style
        } else {
            style.with_border(BorderStyle::new(border, border_width))
        }
    }

    pub(crate) fn mask_style(&self) -> MaskedInputStyle {
        let mut style =
            MaskedInputStyle::new(self.theme.input_mask_color.unwrap_or(self.theme.foreground));
        let scale = self.render_scale_i32();
        style.bullet_size = style.bullet_size.saturating_mul(scale);
        style.spacing = style.spacing.saturating_mul(scale);
        style.horizontal_padding = style.horizontal_padding.saturating_mul(scale);
        style
    }

    pub(crate) fn avatar_style(&self) -> AvatarStyle {
        let ring_width = self.theme.avatar_ring_width.unwrap_or(2).clamp(0, 12);
        let ring = if let Some(ring_color) = self.theme.avatar_ring_color {
            ring_color
        } else if self.focused {
            avatar_ring_color(self.theme.input_border, 108)
        } else {
            avatar_ring_color(self.theme.foreground, 54)
        };
        let background = self.theme.avatar_background;

        let placeholder = self
            .theme
            .avatar_icon_color
            .unwrap_or(self.theme.foreground)
            .with_alpha(224);
        let mut style = AvatarStyle::new(background, placeholder);
        if let Some(radius) = self.theme.avatar_radius {
            style = style.with_radius(radius);
        }
        if ring_width > 0 {
            style = style.with_ring(BorderStyle::new(ring, ring_width));
        }
        if let Some(padding) = self.theme.avatar_placeholder_padding {
            style = style.with_placeholder_padding(padding);
        }
        style
    }

    pub(crate) fn toggle_style(&self) -> IconStyle {
        let interaction_alpha = if self.reveal_toggle_pressed {
            255
        } else if self.reveal_toggle_hovered || self.reveal_secret {
            236
        } else {
            184
        };
        let base = self.theme.eye_icon_color.unwrap_or(self.theme.foreground);
        let alpha = eye_icon_alpha(base.alpha, interaction_alpha);
        IconStyle::new(base.with_alpha(alpha)).with_padding(self.scaled_px(4))
    }

    pub(crate) fn caps_lock_icon_style(&self) -> IconStyle {
        let base = self.theme.caps_lock_color.unwrap_or(self.theme.foreground);
        let alpha = if base.alpha == u8::MAX {
            percent_to_alpha(72)
        } else {
            base.alpha
        };
        IconStyle::new(base.with_alpha(alpha)).with_padding(self.scaled_px(4))
    }
}
