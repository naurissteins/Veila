mod cache;
mod layout;
mod model;
mod styles;
#[cfg(test)]
mod tests;
mod widgets;

pub(super) use cache::TextLayoutCache;

use veila_renderer::SoftwareBuffer;

use self::{
    cache::SceneTextInputs,
    layout::{
        AnchorOffsets, InputPlacement, RoleAnchors, SceneMetrics, role_anchors, top_role_top,
    },
    model::{LayoutRole, SceneModel, SceneSection, SceneTextBlocks, SceneWidget},
    widgets::{
        InputWidget, NowPlayingWidget, draw_avatar_widget, draw_centered_block,
        draw_centered_clock_widget, draw_input_content, draw_input_shell, draw_now_playing_widget,
        draw_top_right_block, draw_top_right_icon_chip, draw_weather_widget, input_toggle_hitbox,
        top_right_chip_diameter,
    },
};
use super::{ShellState, ShellStatus};

const NOW_PLAYING_RIGHT_PADDING: i32 = 48;
const NOW_PLAYING_BOTTOM_PADDING: i32 = 48;
const NOW_PLAYING_MAX_TEXT_WIDTH: u32 = 240;
const NOW_PLAYING_MIN_TEXT_WIDTH: i32 = 64;

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
        self.render_now_playing_widget(buffer, &layout);
        self.render_top_right_indicators(buffer);
    }

    fn scene_layout(&self, size: veila_renderer::FrameSize) -> SceneLayout {
        let metrics = SceneMetrics::from_frame_with_input_placement(
            size.width as i32,
            size.height as i32,
            self.theme.input_width,
            self.theme.input_height,
            self.theme.avatar_size,
            InputPlacement {
                alignment: self.theme.input_alignment,
                horizontal_padding: self.theme.input_horizontal_padding,
                offset_x: self.theme.input_offset_x,
            },
        );
        let model = SceneModel::standard(
            self.scene_text_blocks(metrics),
            self.theme.input_alignment,
            self.theme.avatar_enabled,
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
            self.theme.input_alignment,
            AnchorOffsets {
                auth_stack: self.theme.auth_stack_offset,
                input_vertical_padding: self.theme.input_vertical_padding,
                input_offset_y: self.theme.input_offset_y,
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
        let clock_meridiem_text = self.clock.meridiem_text();
        let clock_meridiem_style = self.clock_meridiem_text_style(metrics);
        let clock_meridiem_offset_x = self.theme.clock_meridiem_offset_x;
        let clock_meridiem_offset_y = self.theme.clock_meridiem_offset_y;
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
                clock_text: self.theme.clock_enabled.then_some(clock_text),
                clock_style,
                clock_meridiem_text: self
                    .theme
                    .clock_enabled
                    .then_some(())
                    .and(clock_meridiem_text),
                clock_meridiem_style,
                clock_meridiem_offset_x,
                clock_meridiem_offset_y,
                date_text: self.theme.date_enabled.then_some(date_text),
                date_style,
                username_text: self.theme.username_enabled.then_some(()).and(username_text),
                username_style,
                placeholder_text: self
                    .theme
                    .placeholder_enabled
                    .then_some(self.hint_text.as_str()),
                placeholder_style,
                status_text: self
                    .theme
                    .status_enabled
                    .then_some(())
                    .and(status_text.as_deref()),
                status_style,
                weather_temperature_text: if self.theme.weather_enabled {
                    weather.map(|weather| weather.temperature_text.as_str())
                } else {
                    None
                },
                weather_temperature_style,
                weather_location_text: if self.theme.weather_enabled {
                    weather.map(|weather| weather.location.as_str())
                } else {
                    None
                },
                weather_location_style,
                weather_icon: if self.theme.weather_enabled {
                    weather.map(|weather| weather.icon)
                } else {
                    None
                },
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
            SceneWidget::Clock(block) if dynamic => {
                draw_centered_clock_widget(buffer, metrics.center_x, y, block);
            }
            SceneWidget::Date(block) | SceneWidget::Status(block) if dynamic => {
                let center_x = if matches!(section.widget, SceneWidget::Status(_)) {
                    metrics.auth_center_x
                } else {
                    metrics.center_x
                };
                draw_centered_block(buffer, center_x, y, block);
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

    fn render_top_right_indicators(&self, buffer: &mut SoftwareBuffer) {
        let keyboard_block = if self.theme.keyboard_enabled {
            self.keyboard_layout_label.as_deref().map(|label| {
                self.text_layout_cache.borrow_mut().keyboard_layout_block(
                    label,
                    self.keyboard_layout_text_style(),
                    120,
                )
            })
        } else {
            None
        };
        let keyboard_chip_diameter = keyboard_block.as_ref().map(|block| {
            top_right_chip_diameter(
                self.theme.keyboard_background_size,
                block.width as i32,
                block.height as i32,
            )
        });
        let row_gap = self.theme.battery_gap.unwrap_or(8).clamp(0, 64);

        if let Some(block) = keyboard_block.as_ref() {
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
                block,
            );
        }

        if self.theme.battery_enabled
            && let Some(battery) = self.battery.as_ref()
        {
            let battery_icon_size = self.theme.battery_size.unwrap_or(18).clamp(12, 96);
            let right_padding = 32
                + keyboard_chip_diameter.unwrap_or(0)
                + if keyboard_chip_diameter.is_some() {
                    row_gap
                } else {
                    0
                };
            let y = (top_role_top(buffer.size().height as i32, self.theme.header_top_offset) - 10
                + self.theme.battery_top_offset.unwrap_or(0))
            .max(8);
            let icon_style = veila_renderer::icon::IconStyle::new(
                self.theme
                    .battery_color
                    .unwrap_or(self.theme.foreground)
                    .with_alpha(
                        self.theme
                            .battery_opacity
                            .map(styles::percent_to_alpha)
                            .unwrap_or(u8::MAX),
                    ),
            );
            draw_top_right_icon_chip(
                buffer,
                right_padding,
                self.theme.battery_right_offset.unwrap_or(0),
                y,
                self.theme.battery_background_color,
                self.theme.battery_background_size,
                battery.icon,
                icon_style,
                battery_icon_size,
            );
        }
    }

    fn render_now_playing_widget(&self, buffer: &mut SoftwareBuffer, layout: &SceneLayout) {
        let fade_progress = self.now_playing_fade_progress();
        if !self.theme.now_playing_enabled
            || (self.now_playing.is_none()
                && self
                    .now_playing_transition
                    .as_ref()
                    .and_then(|transition| transition.previous.as_ref())
                    .is_none())
        {
            return;
        }

        let artwork_size = self
            .theme
            .now_playing_artwork_size
            .unwrap_or(56)
            .clamp(32, 160);
        let content_gap = self
            .theme
            .now_playing_content_gap
            .unwrap_or(widgets::NOW_PLAYING_CONTENT_GAP)
            .clamp(0, 96);
        let now_playing_width = self
            .theme
            .now_playing_width
            .map(|width| width.clamp(96, 640));
        let text_max_width = now_playing_width
            .map(|width| {
                (width - artwork_size - content_gap).max(NOW_PLAYING_MIN_TEXT_WIDTH) as u32
            })
            .unwrap_or(NOW_PLAYING_MAX_TEXT_WIDTH);
        let base_bottom_padding = self
            .theme
            .now_playing_bottom_padding
            .unwrap_or(NOW_PLAYING_BOTTOM_PADDING)
            .clamp(0, 512);
        let bottom_padding = if self.theme.weather_alignment
            == veila_common::WeatherAlignment::Right
            && layout
                .model
                .sections_for_role(LayoutRole::Footer)
                .next()
                .is_some()
        {
            (buffer.size().height as i32 - layout.anchors.footer_y + 24).max(base_bottom_padding)
        } else {
            base_bottom_padding
        };

        if let Some(transition) = self.now_playing_transition.as_ref()
            && let Some(previous) = transition.previous.as_ref()
        {
            let fade_out = 100u8.saturating_sub(fade_progress.unwrap_or(100));
            self.draw_now_playing_snapshot(
                buffer,
                previous,
                artwork_size,
                content_gap,
                now_playing_width,
                text_max_width,
                bottom_padding,
                fade_out,
            );
        }

        if let Some(now_playing) = self.now_playing.as_ref() {
            let fade_in = fade_progress.unwrap_or(100);
            self.draw_now_playing_snapshot(
                buffer,
                now_playing,
                artwork_size,
                content_gap,
                now_playing_width,
                text_max_width,
                bottom_padding,
                fade_in,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_now_playing_snapshot(
        &self,
        buffer: &mut SoftwareBuffer,
        now_playing: &super::NowPlayingWidgetData,
        artwork_size: i32,
        content_gap: i32,
        now_playing_width: Option<i32>,
        text_max_width: u32,
        bottom_padding: i32,
        opacity_scale: u8,
    ) {
        let mut text_layout_cache = self.text_layout_cache.borrow_mut();
        let title = apply_block_opacity(
            text_layout_cache.now_playing_title_block(
                &now_playing.title,
                self.now_playing_title_text_style(),
                text_max_width,
            ),
            opacity_scale,
        );
        let artist = now_playing.artist.as_deref().map(|artist| {
            apply_block_opacity(
                text_layout_cache.now_playing_artist_block(
                    artist,
                    self.now_playing_artist_text_style(),
                    text_max_width,
                ),
                opacity_scale,
            )
        });

        draw_now_playing_widget(
            buffer,
            NowPlayingWidget {
                artwork: now_playing.artwork.as_ref(),
                title: &title,
                artist: artist.as_ref(),
                artwork_opacity: combine_optional_opacity(
                    self.theme.now_playing_artwork_opacity,
                    opacity_scale,
                ),
                artwork_size,
                artwork_radius: self
                    .theme
                    .now_playing_artwork_radius
                    .unwrap_or(12)
                    .clamp(0, 80),
                width: now_playing_width,
                content_gap,
                text_gap: self
                    .theme
                    .now_playing_text_gap
                    .unwrap_or(widgets::NOW_PLAYING_TEXT_GAP)
                    .clamp(0, 64),
                right_padding: self
                    .theme
                    .now_playing_right_padding
                    .unwrap_or(NOW_PLAYING_RIGHT_PADDING)
                    .clamp(0, 512),
                bottom_padding,
                right_offset: self
                    .theme
                    .now_playing_right_offset
                    .unwrap_or(0)
                    .clamp(-512, 512),
                bottom_offset: self
                    .theme
                    .now_playing_bottom_offset
                    .unwrap_or(0)
                    .clamp(-512, 512),
            },
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
                return if self.theme.eye_enabled {
                    input_toggle_hitbox(layout.metrics.input_rect(y))
                } else {
                    veila_renderer::shape::Rect::new(0, 0, 0, 0)
                };
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

fn apply_block_opacity(
    mut block: veila_renderer::text::TextBlock,
    opacity_scale: u8,
) -> veila_renderer::text::TextBlock {
    block.style.color = block.style.color.with_alpha(
        ((u16::from(block.style.color.alpha) * u16::from(opacity_scale.min(100))) / 100) as u8,
    );
    block
}

fn combine_optional_opacity(base: Option<u8>, scale: u8) -> Option<u8> {
    Some(((u16::from(base.unwrap_or(100).min(100)) * u16::from(scale.min(100))) / 100) as u8)
}
