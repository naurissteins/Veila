mod cache;
mod layout;
mod model;
mod styles;
#[cfg(test)]
mod tests;
mod widgets;

pub(super) use cache::TextLayoutCache;

use veila_renderer::{SoftwareBuffer, text::TextStyle};

use self::{
    cache::SceneTextInputs,
    layout::{AnchorOffsets, RoleAnchors, SceneMetrics, role_anchors, top_role_top},
    model::{LayoutRole, SceneModel, SceneSection, SceneTextBlocks, SceneWidget},
    widgets::{
        InputWidget, draw_avatar_widget, draw_centered_block, draw_input_content, draw_input_shell,
        draw_top_right_block, draw_weather_widget, input_toggle_hitbox,
    },
};
use super::{ShellState, ShellStatus};

#[derive(Debug, Clone)]
struct SceneLayout {
    metrics: SceneMetrics,
    model: SceneModel,
    anchors: RoleAnchors,
}

impl ShellState {
    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_overlay(&self, buffer: &mut SoftwareBuffer) {
        self.render_static_overlay(buffer);
        self.render_dynamic_overlay(buffer);
    }

    pub fn render_static_overlay(&self, buffer: &mut SoftwareBuffer) {
        let layout = self.scene_layout(buffer.size());
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Hero,
            layout.anchors.hero_y,
            false,
        );
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Auth,
            layout.anchors.auth_y,
            false,
        );
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Footer,
            layout.anchors.footer_y,
            false,
        );
    }

    pub fn render_dynamic_overlay(&self, buffer: &mut SoftwareBuffer) {
        let layout = self.scene_layout(buffer.size());
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Hero,
            layout.anchors.hero_y,
            true,
        );
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Auth,
            layout.anchors.auth_y,
            true,
        );
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Footer,
            layout.anchors.footer_y,
            true,
        );
        self.render_keyboard_layout_indicator(buffer);
    }

    fn scene_layout(&self, size: veila_renderer::FrameSize) -> SceneLayout {
        let metrics = SceneMetrics::from_frame(
            size.width as i32,
            size.height as i32,
            self.theme.input_width,
            self.theme.input_height,
            self.theme.avatar_size,
        );
        let model = SceneModel::standard(
            self.scene_text_blocks(metrics),
            self.theme.clock_gap,
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
            AnchorOffsets {
                auth_stack: self.theme.auth_stack_offset,
                header_top: self.theme.header_top_offset,
                weather_bottom_padding: self.theme.weather_bottom_padding,
            },
        );

        SceneLayout {
            metrics,
            model,
            anchors,
        }
    }

    fn render_role(
        &self,
        buffer: &mut SoftwareBuffer,
        layout: &SceneLayout,
        role: LayoutRole,
        start_y: i32,
        dynamic: bool,
    ) {
        let mut y = start_y;

        for section in layout.model.sections_for_role(role) {
            self.render_section(buffer, layout.metrics, section, y, dynamic);
            y += section.height(layout.metrics, &self.status) + section.gap_after;
        }
    }

    fn scene_text_blocks(&self, metrics: SceneMetrics) -> SceneTextBlocks {
        let clock_text = self.clock.time_text();
        let clock_style = self.clock_text_style(metrics);
        let date_text = self.clock.date_text();
        let date_style = self.date_text_style();
        let username_text = self.username_text.as_deref();
        let username_style = self.username_text_style();
        let placeholder_style = self.placeholder_text_style();
        let status_text = self.status_text();
        let status_style = self.status_text_style();
        let weather = self.weather.as_ref();
        let weather_temperature_style = self.weather_temperature_text_style();
        let weather_location_style = self.weather_location_text_style();

        self.text_layout_cache
            .borrow_mut()
            .scene_text_blocks(SceneTextInputs {
                clock_text,
                clock_style,
                date_text,
                date_style,
                username_text,
                username_style,
                placeholder_text: &self.hint_text,
                placeholder_style,
                status_text: status_text.as_deref(),
                status_style,
                weather_temperature_text: weather.map(|weather| weather.temperature_text.as_str()),
                weather_temperature_style,
                weather_location_text: weather.map(|weather| weather.location.as_str()),
                weather_location_style,
                weather_icon: weather.map(|weather| weather.icon),
                weather_icon_size: self.theme.weather_icon_size,
                weather_icon_gap: self.theme.weather_icon_gap,
                weather_location_gap: self.theme.weather_location_gap,
                weather_icon_opacity: self.theme.weather_icon_opacity,
                weather_horizontal_padding: self.theme.weather_horizontal_padding,
                weather_alignment: self.theme.weather_alignment,
                weather_left_offset: self.theme.weather_left_offset,
                weather_bottom_offset: self.theme.weather_bottom_offset,
                metrics,
            })
    }

    fn render_section(
        &self,
        buffer: &mut SoftwareBuffer,
        metrics: SceneMetrics,
        section: &SceneSection,
        y: i32,
        dynamic: bool,
    ) {
        match &section.widget {
            SceneWidget::Clock(block) | SceneWidget::Date(block) | SceneWidget::Status(block)
                if dynamic =>
            {
                draw_centered_block(buffer, metrics.center_x, y, block);
            }
            SceneWidget::Username(block) if !dynamic => {
                draw_centered_block(buffer, metrics.center_x, y, block);
            }
            SceneWidget::Avatar if !dynamic => {
                draw_avatar_widget(
                    buffer,
                    &self.avatar,
                    metrics.center_x,
                    y,
                    metrics.avatar_size as u32,
                    self.avatar_style(),
                );
            }
            SceneWidget::Weather(weather) if !dynamic => {
                draw_weather_widget(buffer, y, weather);
            }
            SceneWidget::Input(placeholder) => {
                let caps_lock_indicator =
                    if dynamic && self.caps_lock_active {
                        Some(self.text_layout_cache.borrow_mut().caps_lock_block(
                            self.caps_lock_text_style(),
                            metrics.input_width as u32,
                        ))
                    } else {
                        None
                    };
                let revealed_secret = if self.reveal_secret && !self.secret.is_empty() {
                    Some(self.text_layout_cache.borrow_mut().revealed_secret_block(
                        &self.secret,
                        TextStyle::new(self.theme.foreground.with_alpha(236), 2),
                        metrics.input_width.saturating_sub(92) as u32,
                    ))
                } else {
                    None
                };
                let widget = InputWidget {
                    rect: metrics.input_rect(y),
                    secret_len: self.secret.chars().count(),
                    focused: self.focused,
                    shell_style: self.input_style(),
                    mask_style: self.mask_style(),
                    placeholder: Some(placeholder.clone()),
                    revealed_secret,
                    reveal_secret: self.reveal_secret,
                    toggle_hovered: self.reveal_toggle_hovered,
                    toggle_pressed: self.reveal_toggle_pressed,
                    toggle_style: self.toggle_style(),
                    caps_lock_indicator,
                };
                if dynamic {
                    draw_input_content(buffer, &widget);
                } else {
                    draw_input_shell(buffer, widget.rect, widget.shell_style);
                }
            }
            _ => {}
        }
    }

    fn render_keyboard_layout_indicator(&self, buffer: &mut SoftwareBuffer) {
        let Some(label) = self.keyboard_layout_label.as_deref() else {
            return;
        };

        let block = self.text_layout_cache.borrow_mut().keyboard_layout_block(
            label,
            self.keyboard_layout_text_style(),
            120,
        );
        let y = (top_role_top(buffer.size().height as i32, self.theme.header_top_offset) - 10
            + self.theme.keyboard_top_offset.unwrap_or(0))
        .max(8);
        draw_top_right_block(
            buffer,
            32,
            self.theme.keyboard_right_offset.unwrap_or(0),
            y,
            self.theme.keyboard_background_color,
            self.theme.keyboard_background_size,
            &block,
        );
    }

    pub(super) fn reveal_toggle_rect_for_frame(
        &self,
        frame_width: i32,
        frame_height: i32,
    ) -> veila_renderer::shape::Rect {
        let layout = self.scene_layout(veila_renderer::FrameSize::new(
            frame_width.max(1) as u32,
            frame_height.max(1) as u32,
        ));
        let mut y = layout.anchors.auth_y;

        for section in layout.model.sections_for_role(LayoutRole::Auth) {
            if matches!(section.widget, SceneWidget::Input(_)) {
                return input_toggle_hitbox(layout.metrics.input_rect(y));
            }
            y += section.height(layout.metrics, &self.status) + section.gap_after;
        }

        veila_renderer::shape::Rect::new(0, 0, 0, 0)
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
