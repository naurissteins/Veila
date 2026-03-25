mod layout;
mod model;
mod widgets;

use veila_renderer::{
    ClearColor, SoftwareBuffer,
    avatar::AvatarStyle,
    icon::IconStyle,
    masked::MaskedInputStyle,
    shape::{BorderStyle, PillStyle},
    text::{TextStyle, fit_wrapped_text},
};

use self::{
    layout::{SceneMetrics, role_anchors},
    model::{LayoutRole, SceneModel, SceneSection, SceneTextBlocks, SceneWidget},
    widgets::{
        InputWidget, draw_avatar_widget, draw_centered_block, draw_input_widget,
        input_toggle_hitbox,
    },
};
use super::{ShellState, ShellStatus};

const MAX_HEADER_TEXT_SCALE: u32 = 24;

impl ShellState {
    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_overlay(&self, buffer: &mut SoftwareBuffer) {
        let size = buffer.size();
        let metrics = SceneMetrics::from_frame(
            size.width as i32,
            size.height as i32,
            self.theme.input_width,
            self.theme.input_height,
            self.theme.avatar_size,
        );
        let model = SceneModel::standard(
            self.scene_text_blocks(metrics),
            self.theme.avatar_gap,
            self.theme.username_gap,
            self.theme.status_gap,
        );
        let anchors = role_anchors(
            size.height as i32,
            model.anchor_height_for_role(LayoutRole::Hero, metrics, &self.status),
            model.anchor_height_for_role(LayoutRole::Auth, metrics, &self.status),
            model.total_height_for_role(LayoutRole::Auth, metrics, &self.status),
            model.total_height_for_role(LayoutRole::Footer, metrics, &self.status),
        );

        self.render_role(buffer, metrics, &model, LayoutRole::Hero, anchors.hero_y);
        self.render_role(buffer, metrics, &model, LayoutRole::Auth, anchors.auth_y);
        self.render_role(
            buffer,
            metrics,
            &model,
            LayoutRole::Footer,
            anchors.footer_y,
        );
    }

    fn render_role(
        &self,
        buffer: &mut SoftwareBuffer,
        metrics: SceneMetrics,
        model: &SceneModel,
        role: LayoutRole,
        start_y: i32,
    ) {
        let mut y = start_y;

        for section in model.sections_for_role(role) {
            self.render_section(buffer, metrics, section, y);
            y += section.height(metrics, &self.status) + section.gap_after;
        }
    }

    fn scene_text_blocks(&self, metrics: SceneMetrics) -> SceneTextBlocks {
        SceneTextBlocks {
            clock: fit_wrapped_text(
                self.clock.time_text(),
                self.clock_text_style(metrics),
                metrics.clock_width,
                3,
            ),
            date: fit_wrapped_text(
                self.clock.date_text(),
                self.date_text_style(),
                metrics.clock_width,
                1,
            ),
            username: self.username_text.as_ref().map(|username| {
                fit_wrapped_text(
                    username,
                    self.username_text_style(),
                    metrics.content_width,
                    1,
                )
            }),
            placeholder: fit_wrapped_text(
                &self.hint_text,
                self.placeholder_text_style(),
                metrics.input_width.saturating_sub(48) as u32,
                1,
            ),
            status: self.status_text().map(|text| {
                fit_wrapped_text(&text, self.status_text_style(), metrics.content_width, 1)
            }),
        }
    }

    fn render_section(
        &self,
        buffer: &mut SoftwareBuffer,
        metrics: SceneMetrics,
        section: &SceneSection,
        y: i32,
    ) {
        match &section.widget {
            SceneWidget::Clock(block)
            | SceneWidget::Date(block)
            | SceneWidget::Username(block)
            | SceneWidget::Status(block) => {
                draw_centered_block(buffer, metrics.center_x, y, block);
            }
            SceneWidget::Avatar => {
                draw_avatar_widget(
                    buffer,
                    &self.avatar,
                    metrics.center_x,
                    y,
                    metrics.avatar_size as u32,
                    self.avatar_style(),
                );
            }
            SceneWidget::Input(placeholder) => {
                let revealed_secret = if self.reveal_secret && !self.secret.is_empty() {
                    Some(fit_wrapped_text(
                        &self.secret,
                        TextStyle::new(self.theme.foreground.with_alpha(236), 2),
                        metrics.input_width.saturating_sub(92) as u32,
                        1,
                    ))
                } else {
                    None
                };
                let widget = InputWidget {
                    rect: metrics.input_rect(y),
                    secret_len: self.secret.chars().count(),
                    focused: self.focused,
                    shell_style: self.input_style(),
                    mask_style: MaskedInputStyle::new(self.theme.foreground),
                    placeholder: Some(placeholder.clone()),
                    revealed_secret,
                    reveal_secret: self.reveal_secret,
                    toggle_hovered: self.reveal_toggle_hovered,
                    toggle_pressed: self.reveal_toggle_pressed,
                    toggle_style: self.toggle_style(),
                };
                draw_input_widget(buffer, &widget);
            }
        }
    }

    pub(super) fn reveal_toggle_rect_for_frame(
        &self,
        frame_width: i32,
        frame_height: i32,
    ) -> veila_renderer::shape::Rect {
        let size = frame_width.max(1);
        let metrics = SceneMetrics::from_frame(
            size,
            frame_height.max(1),
            self.theme.input_width,
            self.theme.input_height,
            self.theme.avatar_size,
        );
        let model = SceneModel::standard(
            self.scene_text_blocks(metrics),
            self.theme.avatar_gap,
            self.theme.username_gap,
            self.theme.status_gap,
        );
        let anchors = role_anchors(
            frame_height.max(1),
            model.anchor_height_for_role(LayoutRole::Hero, metrics, &self.status),
            model.anchor_height_for_role(LayoutRole::Auth, metrics, &self.status),
            model.total_height_for_role(LayoutRole::Auth, metrics, &self.status),
            model.total_height_for_role(LayoutRole::Footer, metrics, &self.status),
        );
        let mut y = anchors.auth_y;

        for section in model.sections_for_role(LayoutRole::Auth) {
            if matches!(section.widget, SceneWidget::Input(_)) {
                return input_toggle_hitbox(metrics.input_rect(y));
            }
            y += section.height(metrics, &self.status) + section.gap_after;
        }

        veila_renderer::shape::Rect::new(0, 0, 0, 0)
    }

    fn input_style(&self) -> PillStyle {
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

    fn clock_text_style(&self, metrics: SceneMetrics) -> TextStyle {
        TextStyle::new(
            header_color(
                self.theme.clock_color.unwrap_or(self.theme.foreground),
                self.theme.clock_opacity,
                246,
            ),
            self.theme
                .clock_size
                .unwrap_or_else(|| clock_scale(metrics))
                .clamp(1, MAX_HEADER_TEXT_SCALE),
        )
    }

    fn date_text_style(&self) -> TextStyle {
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
    }

    fn username_text_style(&self) -> TextStyle {
        TextStyle::new(
            username_color(
                self.theme.username_color.unwrap_or(self.theme.foreground),
                self.theme.username_opacity,
            ),
            self.theme.username_size.unwrap_or(2).clamp(1, 6),
        )
    }

    fn placeholder_text_style(&self) -> TextStyle {
        TextStyle::new(
            secondary_text_color(
                self.theme.placeholder_color.unwrap_or(self.theme.muted),
                self.theme.placeholder_opacity,
                154,
            ),
            2,
        )
    }

    fn status_text_style(&self) -> TextStyle {
        TextStyle::new(
            secondary_text_color(
                self.theme.status_color.unwrap_or(self.accent_color()),
                self.theme.status_opacity,
                255,
            ),
            2,
        )
    }

    fn avatar_style(&self) -> AvatarStyle {
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

    fn toggle_style(&self) -> IconStyle {
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

    fn accent_color(&self) -> ClearColor {
        match &self.status {
            ShellStatus::Idle => self.theme.input_border.with_alpha(210),
            ShellStatus::Pending => self.theme.pending,
            ShellStatus::Rejected { .. } => self.theme.rejected,
        }
    }

    fn status_text(&self) -> Option<String> {
        match &self.status {
            ShellStatus::Idle => None,
            ShellStatus::Pending => Some(String::from("Checking password")),
            ShellStatus::Rejected {
                displayed_retry_seconds,
                ..
            } => match displayed_retry_seconds {
                Some(retry_seconds) if *retry_seconds > 0 => {
                    Some(format!("Authentication failed, retry in {retry_seconds}s"))
                }
                Some(_) | None => Some(String::from("Authentication failed")),
            },
        }
    }
}

fn clock_scale(metrics: SceneMetrics) -> u32 {
    if metrics.avatar_size < 100 { 4 } else { 5 }
}

fn avatar_background_color(base: ClearColor, opacity_percent: Option<u8>) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => 104,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

fn avatar_ring_color(base: ClearColor, fallback_alpha: u8) -> ClearColor {
    let alpha = if base.alpha == u8::MAX {
        fallback_alpha
    } else {
        base.alpha
    };

    base.with_alpha(alpha)
}

fn username_color(base: ClearColor, opacity_percent: Option<u8>) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => 214,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

fn header_color(base: ClearColor, opacity_percent: Option<u8>, fallback_alpha: u8) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => fallback_alpha,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

fn secondary_text_color(
    base: ClearColor,
    opacity_percent: Option<u8>,
    fallback_alpha: u8,
) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => fallback_alpha,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

fn percent_to_alpha(percent: u8) -> u8 {
    ((u16::from(percent.min(100)) * 255 + 50) / 100) as u8
}

fn styled_alpha(configured_alpha: u8, fallback_alpha: u8) -> u8 {
    if configured_alpha == u8::MAX {
        fallback_alpha
    } else {
        configured_alpha
    }
}

fn eye_icon_alpha(base_alpha: u8, opacity_percent: Option<u8>, interaction_alpha: u8) -> u8 {
    let effective_percent = match opacity_percent {
        Some(percent) => percent.min(100),
        None => ((u16::from(base_alpha) * 100 + 127) / 255) as u8,
    };
    ((u16::from(interaction_alpha) * u16::from(effective_percent) + 50) / 100) as u8
}

#[cfg(test)]
mod tests {
    use super::{ShellState, layout::SceneMetrics};
    use crate::shell::{ShellStatus, ShellTheme};
    use veila_renderer::{ClearColor, FrameSize, SoftwareBuffer};

    #[test]
    fn unfocused_input_style_uses_configured_input_border() {
        let mut shell = ShellState::default();
        shell.set_focus(false);
        let style = shell.input_style();

        assert_eq!(style.fill.alpha, 232);
        assert_eq!(
            style.border.expect("input border").color,
            shell.theme.input_border.with_alpha(210)
        );
    }

    #[test]
    fn default_input_style_uses_input_border() {
        let shell = ShellState::default();
        let style = shell.input_style();

        assert_eq!(
            style.border.expect("default border").color,
            shell.theme.input_border.with_alpha(240)
        );
    }

    #[test]
    fn focused_input_style_uses_input_border() {
        let mut shell = ShellState::default();
        shell.set_focus(true);
        let style = shell.input_style();

        assert_eq!(
            style.border.expect("focused border").color,
            shell.theme.input_border.with_alpha(240)
        );
    }

    #[test]
    fn explicit_input_alpha_is_preserved() {
        let theme = ShellTheme {
            input: ClearColor::rgba(96, 164, 255, 51),
            input_border: ClearColor::rgba(96, 164, 255, 64),
            ..ShellTheme::default()
        };
        let mut shell = ShellState::new(theme, None, None, true);
        shell.set_focus(false);
        let style = shell.input_style();

        assert_eq!(style.fill.alpha, 51);
        assert_eq!(style.border.expect("input border").color.alpha, 64);
    }

    #[test]
    fn input_style_uses_configured_radius() {
        let theme = ShellTheme {
            input_radius: 18,
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.input_style();

        assert_eq!(style.radius, 18);
    }

    #[test]
    fn input_style_uses_configured_border_width() {
        let theme = ShellTheme {
            input_border_width: Some(4),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.input_style();

        assert_eq!(style.border.expect("input border").thickness, 4);
    }

    #[test]
    fn input_style_allows_disabling_border() {
        let theme = ShellTheme {
            input_border_width: Some(0),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.input_style();

        assert!(style.border.is_none());
    }

    #[test]
    fn explicit_input_opacity_is_preserved_without_style_boost() {
        let theme = ShellTheme {
            input: ClearColor::rgba(255, 255, 255, 26),
            input_border: ClearColor::rgba(255, 255, 255, 31),
            ..ShellTheme::default()
        };
        let mut shell = ShellState::new(theme, None, None, true);
        shell.set_focus(false);
        let style = shell.input_style();

        assert_eq!(style.fill.alpha, 26);
        assert_eq!(style.border.expect("input border").color.alpha, 31);
    }

    #[test]
    fn avatar_style_uses_configured_placeholder_padding() {
        let theme = ShellTheme {
            avatar_placeholder_padding: Some(16),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert_eq!(style.placeholder_padding, Some(16));
    }

    #[test]
    fn avatar_style_uses_configured_icon_color() {
        let theme = ShellTheme {
            avatar_icon_color: Some(ClearColor::opaque(232, 238, 249)),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert_eq!(style.placeholder, ClearColor::rgba(232, 238, 249, 224));
    }

    #[test]
    fn toggle_style_uses_configured_eye_icon_color() {
        let theme = ShellTheme {
            eye_icon_color: Some(ClearColor::opaque(244, 248, 255)),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.toggle_style();

        assert_eq!(style.color, ClearColor::rgba(244, 248, 255, 184));
    }

    #[test]
    fn toggle_style_scales_alpha_with_configured_eye_icon_opacity() {
        let theme = ShellTheme {
            eye_icon_color: Some(ClearColor::opaque(244, 248, 255)),
            eye_icon_opacity: Some(50),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.toggle_style();

        assert_eq!(style.color, ClearColor::rgba(244, 248, 255, 92));
    }

    #[test]
    fn toggle_style_preserves_explicit_eye_icon_alpha_when_unset() {
        let theme = ShellTheme {
            eye_icon_color: Some(ClearColor::rgba(244, 248, 255, 128)),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.toggle_style();

        assert_eq!(style.color.alpha, 92);
    }

    #[test]
    fn avatar_style_uses_configured_ring_width() {
        let theme = ShellTheme {
            avatar_ring_width: Some(4),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert_eq!(style.ring.expect("avatar ring").thickness, 4);
    }

    #[test]
    fn avatar_style_uses_configured_ring_color() {
        let theme = ShellTheme {
            avatar_ring_color: Some(ClearColor::opaque(148, 178, 255)),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert_eq!(
            style.ring.expect("avatar ring").color,
            ClearColor::rgba(148, 178, 255, 108)
        );
    }

    #[test]
    fn avatar_style_preserves_explicit_ring_alpha() {
        let theme = ShellTheme {
            avatar_ring_color: Some(ClearColor::rgba(148, 178, 255, 48)),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert_eq!(style.ring.expect("avatar ring").color.alpha, 48);
    }

    #[test]
    fn avatar_style_allows_disabling_ring() {
        let theme = ShellTheme {
            avatar_ring_width: Some(0),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert!(style.ring.is_none());
    }

    #[test]
    fn avatar_style_uses_configured_background_opacity() {
        let theme = ShellTheme {
            avatar_background: ClearColor::rgba(24, 30, 42, 255),
            avatar_background_opacity: Some(36),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert_eq!(style.background.alpha, 92);
    }

    #[test]
    fn avatar_style_preserves_explicit_panel_alpha_when_unset() {
        let theme = ShellTheme {
            avatar_background: ClearColor::rgba(24, 30, 42, 80),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.avatar_style();

        assert_eq!(style.background.alpha, 80);
    }

    #[test]
    fn scene_metrics_use_configured_avatar_size() {
        let theme = ShellTheme {
            avatar_size: Some(88),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(1280, 720)).expect("buffer");

        shell.render_overlay(&mut buffer);

        let metrics = SceneMetrics::from_frame(
            1280,
            720,
            shell.theme.input_width,
            shell.theme.input_height,
            shell.theme.avatar_size,
        );
        assert_eq!(metrics.avatar_size, 88);
    }

    #[test]
    fn username_style_uses_configured_opacity_and_size() {
        let theme = ShellTheme {
            foreground: ClearColor::rgba(240, 244, 250, 255),
            username_opacity: Some(72),
            username_size: Some(3),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.username_text_style();

        assert_eq!(style.color.alpha, 184);
        assert_eq!(style.scale, 3);
    }

    #[test]
    fn username_style_uses_configured_color() {
        let theme = ShellTheme {
            username_color: Some(ClearColor::opaque(215, 227, 255)),
            username_opacity: Some(72),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.username_text_style();

        assert_eq!(style.color.red, 215);
        assert_eq!(style.color.green, 227);
        assert_eq!(style.color.blue, 255);
        assert_eq!(style.color.alpha, 184);
    }

    #[test]
    fn username_style_preserves_explicit_foreground_alpha_when_unset() {
        let theme = ShellTheme {
            foreground: ClearColor::rgba(240, 244, 250, 90),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.username_text_style();

        assert_eq!(style.color.alpha, 90);
        assert_eq!(style.scale, 2);
    }

    #[test]
    fn clock_style_uses_configured_opacity() {
        let theme = ShellTheme {
            foreground: ClearColor::rgba(240, 244, 250, 255),
            clock_opacity: Some(96),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

        assert_eq!(style.color.alpha, 245);
        assert_eq!(style.scale, 5);
    }

    #[test]
    fn clock_style_uses_configured_color() {
        let theme = ShellTheme {
            clock_color: Some(ClearColor::opaque(248, 251, 255)),
            clock_opacity: Some(96),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

        assert_eq!(style.color.red, 248);
        assert_eq!(style.color.green, 251);
        assert_eq!(style.color.blue, 255);
        assert_eq!(style.color.alpha, 245);
    }

    #[test]
    fn date_style_uses_configured_opacity() {
        let theme = ShellTheme {
            foreground: ClearColor::rgba(240, 244, 250, 255),
            date_opacity: Some(74),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.date_text_style();

        assert_eq!(style.color.alpha, 189);
        assert_eq!(style.scale, 2);
    }

    #[test]
    fn date_style_uses_configured_color() {
        let theme = ShellTheme {
            date_color: Some(ClearColor::opaque(200, 212, 236)),
            date_opacity: Some(74),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.date_text_style();

        assert_eq!(style.color.red, 200);
        assert_eq!(style.color.green, 212);
        assert_eq!(style.color.blue, 236);
        assert_eq!(style.color.alpha, 189);
    }

    #[test]
    fn clock_style_uses_configured_size() {
        let theme = ShellTheme {
            clock_size: Some(4),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

        assert_eq!(style.scale, 4);
    }

    #[test]
    fn clock_style_allows_sizes_above_previous_cap() {
        let theme = ShellTheme {
            clock_size: Some(12),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

        assert_eq!(style.scale, 12);
    }

    #[test]
    fn date_style_uses_configured_size() {
        let theme = ShellTheme {
            date_size: Some(3),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.date_text_style();

        assert_eq!(style.scale, 3);
    }

    #[test]
    fn date_style_allows_sizes_above_previous_cap() {
        let theme = ShellTheme {
            date_size: Some(12),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.date_text_style();

        assert_eq!(style.scale, 12);
    }

    #[test]
    fn header_styles_preserve_explicit_foreground_alpha_when_unset() {
        let theme = ShellTheme {
            foreground: ClearColor::rgba(240, 244, 250, 90),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let clock_style =
            shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));
        let date_style = shell.date_text_style();

        assert_eq!(clock_style.color.alpha, 90);
        assert_eq!(date_style.color.alpha, 90);
    }

    #[test]
    fn scene_metrics_use_configured_input_dimensions() {
        let theme = ShellTheme {
            input_width: Some(280),
            input_height: Some(54),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let metrics = SceneMetrics::from_frame(
            1280,
            720,
            shell.theme.input_width,
            shell.theme.input_height,
            shell.theme.avatar_size,
        );

        assert_eq!(metrics.input_width, 280);
        assert_eq!(metrics.input_height, 54);
    }

    #[test]
    fn placeholder_style_uses_configured_opacity() {
        let theme = ShellTheme {
            muted: ClearColor::rgba(72, 82, 108, 255),
            placeholder_opacity: Some(60),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.placeholder_text_style();

        assert_eq!(style.color.alpha, 153);
        assert_eq!(style.scale, 2);
    }

    #[test]
    fn placeholder_style_uses_configured_color() {
        let theme = ShellTheme {
            placeholder_color: Some(ClearColor::opaque(134, 148, 180)),
            placeholder_opacity: Some(60),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.placeholder_text_style();

        assert_eq!(style.color.red, 134);
        assert_eq!(style.color.green, 148);
        assert_eq!(style.color.blue, 180);
        assert_eq!(style.color.alpha, 153);
    }

    #[test]
    fn status_style_uses_configured_opacity() {
        let theme = ShellTheme {
            input_border: ClearColor::rgba(255, 255, 255, 255),
            status_opacity: Some(88),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.status_text_style();

        assert_eq!(style.color.alpha, 224);
        assert_eq!(style.scale, 2);
    }

    #[test]
    fn status_style_uses_configured_color() {
        let theme = ShellTheme {
            status_color: Some(ClearColor::opaque(255, 224, 160)),
            status_opacity: Some(88),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.status_text_style();

        assert_eq!(style.color.red, 255);
        assert_eq!(style.color.green, 224);
        assert_eq!(style.color.blue, 160);
        assert_eq!(style.color.alpha, 224);
    }

    #[test]
    fn placeholder_style_preserves_explicit_muted_alpha_when_unset() {
        let theme = ShellTheme {
            muted: ClearColor::rgba(72, 82, 108, 90),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let style = shell.placeholder_text_style();

        assert_eq!(style.color.alpha, 90);
    }

    #[test]
    fn status_style_preserves_explicit_pending_alpha_when_unset() {
        let theme = ShellTheme {
            pending: ClearColor::rgba(255, 194, 92, 90),
            ..ShellTheme::default()
        };
        let mut shell = ShellState::new(theme, None, None, true);
        shell.status = ShellStatus::Pending;
        let style = shell.status_text_style();

        assert_eq!(style.color.alpha, 90);
    }
}
