use veila_renderer::{
    avatar::AvatarStyle,
    icon::IconStyle,
    masked::MaskedInputStyle,
    shape::{BorderStyle, PillStyle},
};

use super::{
    super::ShellState,
    color::{avatar_background_color, avatar_ring_color, eye_icon_alpha, styled_alpha},
};

impl ShellState {
    pub(crate) fn input_style(&self) -> PillStyle {
        let border = if self.focused {
            self.theme
                .input_border
                .with_alpha(styled_alpha(self.theme.input_border.alpha, 240))
        } else {
            self.theme
                .input_border
                .with_alpha(styled_alpha(self.theme.input_border.alpha, 210))
        };
        let border_width = self.theme.input_border_width.unwrap_or(2).max(0);

        let style = PillStyle::new(
            self.theme
                .input
                .with_alpha(styled_alpha(self.theme.input.alpha, 232)),
        )
        .with_radius(self.theme.input_radius);

        if border_width == 0 {
            style
        } else {
            style.with_border(BorderStyle::new(border, border_width))
        }
    }

    pub(crate) fn mask_style(&self) -> MaskedInputStyle {
        MaskedInputStyle::new(self.theme.input_mask_color.unwrap_or(self.theme.foreground))
    }

    pub(crate) fn avatar_style(&self) -> AvatarStyle {
        let ring_width = self.theme.avatar_ring_width.unwrap_or(2).clamp(0, 12);
        let ring = if self.focused {
            avatar_ring_color(
                self.theme
                    .avatar_ring_color
                    .unwrap_or(self.theme.input_border),
                108,
            )
        } else {
            avatar_ring_color(
                self.theme
                    .avatar_ring_color
                    .unwrap_or(self.theme.foreground),
                54,
            )
        };
        let background = avatar_background_color(
            self.theme.avatar_background,
            self.theme.avatar_background_opacity,
        );

        let placeholder = self
            .theme
            .avatar_icon_color
            .unwrap_or(self.theme.foreground)
            .with_alpha(224);
        let mut style = AvatarStyle::new(background, placeholder);
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
        let alpha = eye_icon_alpha(base.alpha, self.theme.eye_icon_opacity, interaction_alpha);
        IconStyle::new(base.with_alpha(alpha)).with_padding(4)
    }
}
