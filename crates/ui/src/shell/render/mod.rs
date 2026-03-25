mod layout;
mod model;
mod widgets;

use veila_renderer::{
    ClearColor, SoftwareBuffer,
    avatar::AvatarStyle,
    masked::MaskedInputStyle,
    shape::{BorderStyle, PillStyle},
    text::{TextStyle, fit_wrapped_text},
};

use self::{
    layout::{SceneMetrics, role_anchors},
    model::{LayoutRole, SceneModel, SceneSection, SceneTextBlocks, SceneWidget},
    widgets::{draw_avatar_widget, draw_centered_block, draw_input_widget},
};
use super::{ShellState, ShellStatus};

impl ShellState {
    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_overlay(&self, buffer: &mut SoftwareBuffer) {
        let size = buffer.size();
        let metrics = SceneMetrics::from_frame(size.width as i32, size.height as i32);
        let model = SceneModel::standard(self.scene_text_blocks(metrics));
        let anchors = role_anchors(
            size.height as i32,
            model.total_height_for_role(LayoutRole::Hero, metrics, &self.status),
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
                TextStyle::new(self.theme.foreground.with_alpha(246), clock_scale(metrics)),
                metrics.clock_width,
                3,
            ),
            date: fit_wrapped_text(
                self.clock.date_text(),
                TextStyle::new(self.theme.foreground.with_alpha(188), 2),
                metrics.clock_width,
                1,
            ),
            username: self.username_text.as_ref().map(|username| {
                fit_wrapped_text(
                    username,
                    TextStyle::new(self.theme.foreground.with_alpha(214), 2),
                    metrics.content_width,
                    1,
                )
            }),
            placeholder: fit_wrapped_text(
                &self.hint_text,
                TextStyle::new(self.theme.muted.with_alpha(154), 2),
                metrics.input_width.saturating_sub(48) as u32,
                1,
            ),
            status: self.status_text().map(|text| {
                fit_wrapped_text(
                    &text,
                    TextStyle::new(self.accent_color(), 2),
                    metrics.content_width,
                    1,
                )
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
                draw_input_widget(
                    buffer,
                    metrics.input_rect(y),
                    self.secret.chars().count(),
                    self.focused,
                    self.input_style(),
                    MaskedInputStyle::new(self.theme.foreground),
                    Some(placeholder),
                );
            }
        }
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

        PillStyle::new(
            self.theme
                .input
                .with_alpha(styled_alpha(self.theme.input.alpha, 232)),
        )
        .with_radius(self.theme.input_radius)
        .with_border(BorderStyle::new(border, 2))
    }

    fn avatar_style(&self) -> AvatarStyle {
        let ring = if self.focused {
            self.theme
                .input_border
                .with_alpha(styled_alpha(self.theme.input_border.alpha, 108))
        } else {
            self.theme.foreground.with_alpha(54)
        };

        AvatarStyle::new(
            self.theme.panel.with_alpha(104),
            self.theme.foreground.with_alpha(224),
        )
        .with_ring(BorderStyle::new(ring, 2))
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

fn styled_alpha(configured_alpha: u8, fallback_alpha: u8) -> u8 {
    if configured_alpha == u8::MAX {
        fallback_alpha
    } else {
        configured_alpha
    }
}

#[cfg(test)]
mod tests {
    use super::ShellState;
    use crate::shell::ShellTheme;
    use veila_renderer::ClearColor;

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
}
