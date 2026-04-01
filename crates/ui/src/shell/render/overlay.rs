use veila_renderer::{FrameSize, SoftwareBuffer};

use super::super::{ShellState, ShellStatus};
use super::{
    SceneLayout,
    layout::{LayerPlacement, SceneMetrics, hero_block_x, layer_center_x},
    model::{AuthGroup, LayoutRole, SceneSection, SceneWidget},
    widgets::{
        InputWidget, draw_avatar_widget, draw_block, draw_centered_block, draw_clock_widget,
        draw_input_content, draw_input_shell, draw_weather_widget, input_toggle_hitbox,
    },
};

impl ShellState {
    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_overlay(&self, buffer: &mut SoftwareBuffer) {
        self.render_backdrop_layer(buffer);
        self.render_static_overlay(buffer);
        self.render_dynamic_overlay(buffer);
    }

    pub fn render_static_overlay(&self, buffer: &mut SoftwareBuffer) {
        let layout = self.scene_layout(buffer.size());
        self.render_identity_group(buffer, &layout, false);
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Hero,
            layout.anchors.hero_y,
            false,
        );
        self.render_auth_or_input_group(buffer, &layout, false);
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
        self.render_identity_group(buffer, &layout, true);
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Hero,
            layout.anchors.hero_y,
            true,
        );
        self.render_auth_or_input_group(buffer, &layout, true);
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Footer,
            layout.anchors.footer_y,
            true,
        );
        self.render_now_playing_widget(buffer, &layout);
        self.render_top_right_indicators(buffer);
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

    fn render_identity_group(
        &self,
        buffer: &mut SoftwareBuffer,
        layout: &SceneLayout,
        dynamic: bool,
    ) {
        let Some(start_y) = layout.anchors.identity_y else {
            return;
        };

        let mut y = start_y;
        for section in layout.model.sections_for_auth_group(AuthGroup::Identity) {
            self.render_section(buffer, layout.metrics, section, y, dynamic);
            y += section.height(layout.metrics, &self.status) + section.gap_after;
        }
    }

    fn render_auth_or_input_group(
        &self,
        buffer: &mut SoftwareBuffer,
        layout: &SceneLayout,
        dynamic: bool,
    ) {
        if layout.anchors.identity_y.is_some() {
            let mut y = layout.anchors.auth_y;
            for section in layout.model.sections_for_auth_group(AuthGroup::Input) {
                self.render_section(buffer, layout.metrics, section, y, dynamic);
                y += section.height(layout.metrics, &self.status) + section.gap_after;
            }
        } else {
            self.render_role(
                buffer,
                layout,
                LayoutRole::Auth,
                layout.anchors.auth_y,
                dynamic,
            );
        }
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
            SceneWidget::Clock(block) if dynamic => {
                let layer_center_x = (self.theme.layer_enabled && self.theme.clock_center_in_layer)
                    .then(|| {
                        layer_center_x(
                            buffer.size().width as i32,
                            LayerPlacement {
                                alignment: self.theme.layer_alignment,
                                full_width: self.theme.layer_full_width,
                                width: self.theme.layer_width,
                                full_height: self.theme.layer_full_height,
                                height: self.theme.layer_height,
                                vertical_alignment: self.theme.layer_vertical_alignment,
                                offset_x: self.theme.layer_offset_x,
                                offset_y: self.theme.layer_offset_y,
                                left_padding: self.theme.layer_left_padding,
                                right_padding: self.theme.layer_right_padding,
                                top_padding: self.theme.layer_top_padding,
                                bottom_padding: self.theme.layer_bottom_padding,
                            },
                        )
                    });
                let x = hero_block_x(
                    buffer.size().width as i32,
                    block.width(),
                    self.theme.clock_alignment,
                    layer_center_x,
                    self.theme.clock_offset_x,
                );
                draw_clock_widget(buffer, x, y, block);
            }
            SceneWidget::Date(block) | SceneWidget::Status(block) if dynamic => {
                if matches!(section.widget, SceneWidget::Status(_)) {
                    draw_centered_block(buffer, metrics.auth_center_x, y, block);
                } else {
                    let layer_center_x =
                        (self.theme.layer_enabled && self.theme.clock_center_in_layer).then(|| {
                            layer_center_x(
                                buffer.size().width as i32,
                                LayerPlacement {
                                    alignment: self.theme.layer_alignment,
                                    full_width: self.theme.layer_full_width,
                                    width: self.theme.layer_width,
                                    full_height: self.theme.layer_full_height,
                                    height: self.theme.layer_height,
                                    vertical_alignment: self.theme.layer_vertical_alignment,
                                    offset_x: self.theme.layer_offset_x,
                                    offset_y: self.theme.layer_offset_y,
                                    left_padding: self.theme.layer_left_padding,
                                    right_padding: self.theme.layer_right_padding,
                                    top_padding: self.theme.layer_top_padding,
                                    bottom_padding: self.theme.layer_bottom_padding,
                                },
                            )
                        });
                    let x = hero_block_x(
                        buffer.size().width as i32,
                        block.width as i32,
                        self.theme.clock_alignment,
                        layer_center_x,
                        self.theme.clock_offset_x,
                    );
                    draw_block(buffer, x, y, block);
                }
            }
            SceneWidget::Username(block) if !dynamic => {
                draw_centered_block(buffer, metrics.auth_center_x, y, block);
            }
            SceneWidget::Avatar if !dynamic && self.theme.avatar_enabled => {
                draw_avatar_widget(
                    buffer,
                    &self.avatar,
                    metrics.auth_center_x,
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
                    if dynamic && self.caps_lock_active && self.theme.caps_lock_enabled {
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
                        self.revealed_secret_text_style(),
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
                    placeholder: placeholder.clone(),
                    revealed_secret,
                    reveal_secret: self.reveal_secret,
                    toggle_hovered: self.reveal_toggle_hovered,
                    toggle_pressed: self.reveal_toggle_pressed,
                    show_toggle: self.theme.eye_enabled,
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

    pub(crate) fn reveal_toggle_rect_for_frame(
        &self,
        frame_width: i32,
        frame_height: i32,
    ) -> veila_renderer::shape::Rect {
        let layout = self.scene_layout(FrameSize::new(
            frame_width.max(1) as u32,
            frame_height.max(1) as u32,
        ));
        let mut y = layout.anchors.auth_y;

        if layout.anchors.identity_y.is_some() {
            for section in layout.model.sections_for_auth_group(AuthGroup::Input) {
                if matches!(section.widget, SceneWidget::Input(_)) {
                    return if self.theme.eye_enabled {
                        input_toggle_hitbox(layout.metrics.input_rect(y))
                    } else {
                        veila_renderer::shape::Rect::new(0, 0, 0, 0)
                    };
                }
                y += section.height(layout.metrics, &self.status) + section.gap_after;
            }
        } else {
            for section in layout.model.sections_for_role(LayoutRole::Auth) {
                if matches!(section.widget, SceneWidget::Input(_)) {
                    return if self.theme.eye_enabled {
                        input_toggle_hitbox(layout.metrics.input_rect(y))
                    } else {
                        veila_renderer::shape::Rect::new(0, 0, 0, 0)
                    };
                }
                y += section.height(layout.metrics, &self.status) + section.gap_after;
            }
        }

        veila_renderer::shape::Rect::new(0, 0, 0, 0)
    }

    pub(super) fn status_text(&self) -> Option<String> {
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
