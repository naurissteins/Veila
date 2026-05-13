mod cache;
mod indicators;
mod layout;
mod model;
mod now_playing;
mod overlay;
mod styles;
#[cfg(test)]
mod tests;
mod widgets;

pub(super) use cache::TextLayoutCache;

use std::cell::RefCell;

use veila_common::{BackdropMode, LayerKind};
use veila_common::{BackdropShowWhen, StatusDisplayMode};
use veila_renderer::{
    PixelBuffer,
    layer::{BackdropLayerMode, BackdropLayerShape, BackdropLayerStyle, draw_backdrop_layer},
    shape::{PillStyle, Rect, draw_pill},
};

use self::{
    cache::SceneTextInputs,
    layout::{
        AnchorOffsets, AuthGroupHeights, FooterHeights, RoleAnchorInput, RoleAnchors, SceneMetrics,
        anchored_block_x, anchored_block_y, role_anchors_with_groups,
    },
    model::{AuthGroup, LayoutRole, SceneModel, SceneTextBlocks, StandardSceneConfig},
};
use super::ShellState;
use crate::shell::theme::WidgetPositionTarget;

const NOW_PLAYING_MAX_TEXT_WIDTH: u32 = 318;
const NOW_PLAYING_MIN_TEXT_WIDTH: i32 = 64;

#[derive(Debug, Clone)]
struct SceneLayout {
    metrics: SceneMetrics,
    model: SceneModel,
    anchors: RoleAnchors,
    floating_avatar: bool,
    floating_input: bool,
    floating_input_placeholder: Option<veila_renderer::text::TextBlock>,
    floating_status: Option<veila_renderer::text::TextBlock>,
    floating_status_follows_input: bool,
    floating_username: Option<veila_renderer::text::TextBlock>,
    floating_clock: Option<model::SceneClockBlocks>,
    floating_date: Option<veila_renderer::text::TextBlock>,
    floating_weather: Option<model::SceneWeatherBlocks>,
}

impl ShellState {
    fn scene_layout(&self, size: veila_renderer::FrameSize) -> SceneLayout {
        let metrics = SceneMetrics::new(
            size.width as i32,
            size.height as i32,
            self.theme.input_width,
            self.theme.input_height,
            self.theme.avatar_size,
        );
        let identity_visible = self.identity_visible();
        let input_visible = self.input_visible();
        let text_blocks = self.scene_text_blocks(metrics);
        let status_mode_external = self.theme.status_mode == StatusDisplayMode::External;
        let floating_avatar = self.theme.avatar_enabled && self.theme.avatar_position.is_some();
        let floating_username =
            self.theme.username_enabled && self.theme.username_position.is_some();
        let floating_input = self.theme.input_position.is_some() && input_visible;
        let floating_status_follows_input = status_mode_external
            && input_visible
            && self.theme.input_position.is_some()
            && self.theme.status_position.is_none();
        let floating_status_explicit =
            status_mode_external && input_visible && self.theme.status_position.is_some();
        let clock_in_flow = self.theme.clock_position.is_none();
        let date_in_flow = self.theme.date_position.is_none();
        let avatar_in_flow = !floating_avatar;
        let username_in_flow = !floating_username;
        let input_in_flow = !floating_input;
        let status_in_flow = !floating_status_follows_input && !floating_status_explicit;
        let floating_clock = (!clock_in_flow)
            .then(|| text_blocks.clock.clone())
            .flatten();
        let floating_date = (!date_in_flow).then(|| text_blocks.date.clone()).flatten();
        let floating_weather = text_blocks.weather.clone();
        let model = SceneModel::standard(
            SceneTextBlocks {
                clock: if clock_in_flow {
                    text_blocks.clock.clone()
                } else {
                    None
                },
                date: if date_in_flow {
                    text_blocks.date.clone()
                } else {
                    None
                },
                username: if username_in_flow {
                    text_blocks.username.clone()
                } else {
                    None
                },
                placeholder: if input_in_flow {
                    text_blocks.placeholder.clone()
                } else {
                    None
                },
                status: if status_in_flow {
                    text_blocks.status.clone()
                } else {
                    None
                },
                weather: None,
            },
            StandardSceneConfig {
                identity_visible,
                input_visible: input_visible && input_in_flow,
                avatar_enabled: self.theme.avatar_enabled && avatar_in_flow,
                clock_gap: self.theme.clock_gap,
                avatar_gap: self.theme.avatar_gap,
                username_gap: self.theme.username_gap,
            },
        );
        let anchors = role_anchors_with_groups(RoleAnchorInput {
            frame_height: size.height as i32,
            hero_height: model.anchor_height_for_role(LayoutRole::Hero, metrics, &self.status),
            auth_anchor_height: model.anchor_height_for_role(
                LayoutRole::Auth,
                metrics,
                &self.status,
            ),
            auth_render_height: model.total_height_for_role(
                LayoutRole::Auth,
                metrics,
                &self.status,
            ),
            auth_groups: AuthGroupHeights {
                identity: model.anchor_height_for_auth_group(
                    AuthGroup::Identity,
                    metrics,
                    &self.status,
                ),
                input_anchor: model.anchor_height_for_auth_group(
                    AuthGroup::Input,
                    metrics,
                    &self.status,
                ),
                input_render: model.total_height_for_auth_group(
                    AuthGroup::Input,
                    metrics,
                    &self.status,
                ),
            },
            footer_heights: FooterHeights {
                render: 0,
                clearance: 0,
            },
            offsets: AnchorOffsets {
                clock_alignment: self.theme.clock_alignment,
                clock_offset_y: self.theme.clock_offset_y,
            },
        });

        SceneLayout {
            metrics,
            model,
            anchors,
            floating_avatar,
            floating_input,
            floating_input_placeholder: floating_input
                .then(|| text_blocks.placeholder.clone())
                .flatten(),
            floating_status: (!status_in_flow)
                .then(|| text_blocks.status.clone())
                .flatten(),
            floating_status_follows_input,
            floating_username: floating_username
                .then(|| text_blocks.username.clone())
                .flatten(),
            floating_clock,
            floating_date,
            floating_weather,
        }
    }

    pub fn render_backdrops(&self, buffer: &mut impl PixelBuffer) {
        self.render_backdrops_matching(buffer, |_| true);
    }

    pub fn render_static_backdrops(&self, buffer: &mut impl PixelBuffer) {
        self.render_backdrops_matching(buffer, |show_when| show_when == BackdropShowWhen::Always);
    }

    pub fn render_dynamic_backdrops(&self, buffer: &mut impl PixelBuffer) {
        self.render_backdrops_matching(buffer, |show_when| show_when != BackdropShowWhen::Always);
    }

    pub fn render_backdrops_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        let scale = scale.max(1);
        if scale == 1 {
            self.render_backdrops(buffer);
            return;
        }

        let mut scaled = self.clone();
        scaled.theme = self.theme.scaled_for_render(scale);
        scaled.text_layout_cache = RefCell::new(TextLayoutCache::default());
        scaled.render_backdrops(buffer);
    }

    pub fn render_static_backdrops_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_scaled_theme(scale, |shell| shell.render_static_backdrops(buffer));
    }

    pub fn render_dynamic_backdrops_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_scaled_theme(scale, |shell| shell.render_dynamic_backdrops(buffer));
    }

    fn render_backdrops_matching(
        &self,
        buffer: &mut impl PixelBuffer,
        matches_show_when: impl Fn(BackdropShowWhen) -> bool,
    ) {
        for backdrop in &self.theme.backdrops {
            if !matches_show_when(backdrop.show_when) || !self.backdrop_visible(backdrop) {
                continue;
            }
            let rect = self.backdrop_rect(buffer.size(), backdrop.clone());
            let mode = match backdrop.mode {
                BackdropMode::Solid => BackdropLayerMode::Solid,
                BackdropMode::Blur => BackdropLayerMode::Blur,
            };

            draw_backdrop_layer(
                buffer,
                rect,
                BackdropLayerStyle::new(
                    mode,
                    BackdropLayerShape::Panel,
                    backdrop.color,
                    backdrop.blur_strength,
                    backdrop.radius,
                    backdrop.border_color,
                    backdrop.border_width,
                ),
            );
        }
    }

    fn with_scaled_theme(&self, scale: u32, render: impl FnOnce(&ShellState)) {
        let scale = scale.max(1);
        if scale == 1 {
            render(self);
            return;
        }

        let mut scaled = self.clone();
        scaled.theme = self.theme.scaled_for_render(scale);
        scaled.text_layout_cache = RefCell::new(TextLayoutCache::default());
        render(&scaled);
    }

    pub fn render_layers(&self, buffer: &mut impl PixelBuffer) {
        for (index, layer) in self.theme.layers.iter().enumerate() {
            let max_text_width = layer
                .width
                .map(|width| width.saturating_sub(layer.padding * 2).max(1) as u32)
                .unwrap_or(4_096);
            let style = self.custom_layer_text_style(layer);
            let block = self.text_layout_cache.borrow_mut().custom_layer_block(
                index,
                &layer.text,
                style,
                max_text_width,
                matches!(layer.kind, LayerKind::Icon),
            );
            let icon_bounds = matches!(layer.kind, LayerKind::Icon)
                .then(|| {
                    self.text_layout_cache
                        .borrow_mut()
                        .custom_layer_visible_bounds(index, &layer.text, block.style.clone())
                })
                .flatten();
            let content_width = icon_bounds.map_or(block.width as i32, |bounds| bounds.width());
            let content_height = icon_bounds.map_or(block.height as i32, |bounds| bounds.height());
            let min_width = content_width + layer.padding * 2;
            let min_height = content_height + layer.padding * 2;
            let width = layer.width.unwrap_or(min_width).max(min_width);
            let height = layer.height.unwrap_or(min_height).max(min_height);
            let rect = self.positioned_rect(buffer.size(), layer.position, width, height);

            if let Some(background) = layer.background_color
                && background.alpha > 0
            {
                draw_pill(
                    buffer,
                    rect,
                    PillStyle::new(background).with_radius(layer.radius),
                );
            }

            let (text_x, text_y) = icon_bounds.map_or_else(
                || {
                    (
                        rect.x + (rect.width - content_width) / 2,
                        rect.y + (rect.height - content_height) / 2,
                    )
                },
                |bounds| {
                    (
                        rect.x + rect.width / 2 - (bounds.left + bounds.right) / 2,
                        rect.y + rect.height / 2 - (bounds.top + bounds.bottom) / 2,
                    )
                },
            );
            block.draw(buffer, text_x, text_y);
        }
    }

    pub fn render_layers_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        let scale = scale.max(1);
        if scale == 1 {
            self.render_layers(buffer);
            return;
        }

        let mut scaled = self.clone();
        scaled.theme = self.theme.scaled_for_render(scale);
        scaled.text_layout_cache = RefCell::new(TextLayoutCache::default());
        scaled.render_layers(buffer);
    }

    pub(super) fn backdrop_rect(
        &self,
        size: veila_renderer::FrameSize,
        backdrop: crate::shell::theme::Backdrop,
    ) -> Rect {
        let screen_width = size.width as i32;
        let screen_height = size.height as i32;
        let x = if backdrop.full_width {
            backdrop.inset_left.min(screen_width)
        } else {
            anchored_block_x(
                screen_width,
                backdrop.width,
                backdrop.position.halign,
                backdrop.position.x,
            )
        };
        let y = if backdrop.full_height {
            backdrop.inset_top.min(screen_height)
        } else {
            anchored_block_y(
                screen_height,
                backdrop.height,
                backdrop.position.valign,
                backdrop.position.y,
            )
        };
        let width = if backdrop.full_width {
            (screen_width - backdrop.inset_left - backdrop.inset_right).max(0)
        } else {
            backdrop.width
        };
        let height = if backdrop.full_height {
            (screen_height - backdrop.inset_top - backdrop.inset_bottom).max(0)
        } else {
            backdrop.height
        };

        Rect::new(x, y, width, height)
    }

    pub(super) fn first_backdrop_center_x(&self, size: veila_renderer::FrameSize) -> Option<i32> {
        let backdrop = self.theme.backdrops.first()?.clone();
        let rect = self.backdrop_rect(size, backdrop);
        Some(rect.x + rect.width / 2)
    }

    pub(super) fn positioned_rect(
        &self,
        size: veila_renderer::FrameSize,
        position: crate::shell::theme::WidgetPosition,
        width: i32,
        height: i32,
    ) -> Rect {
        let container = self.position_container_rect(size, position);
        Rect::new(
            container.x + anchored_block_x(container.width, width, position.halign, position.x),
            container.y + anchored_block_y(container.height, height, position.valign, position.y),
            width,
            height,
        )
    }

    fn position_container_rect(
        &self,
        size: veila_renderer::FrameSize,
        position: crate::shell::theme::WidgetPosition,
    ) -> Rect {
        match position.target {
            WidgetPositionTarget::Screen => Rect::new(0, 0, size.width as i32, size.height as i32),
            WidgetPositionTarget::Backdrop(index) => self
                .theme
                .backdrops
                .get(index)
                .cloned()
                .map(|backdrop| self.backdrop_rect(size, backdrop))
                .unwrap_or_else(|| Rect::new(0, 0, size.width as i32, size.height as i32)),
        }
    }

    fn scene_text_blocks(&self, metrics: SceneMetrics) -> SceneTextBlocks {
        let identity_visible = self.identity_visible();
        let input_visible = self.input_visible();
        let status_mode_external = self.theme.status_mode == StatusDisplayMode::External;
        let clock_text = self.clock.primary_text(self.theme.clock_style);
        let clock_secondary_text = self.clock.secondary_text(self.theme.clock_style);
        let clock_style = self.clock_text_style(metrics);
        let clock_meridiem_text = self.clock.meridiem_text();
        let clock_meridiem_style = self.clock_meridiem_text_style(metrics);
        let clock_meridiem_x = self.theme.clock_meridiem_x;
        let clock_meridiem_y = self.theme.clock_meridiem_y;
        let date_text = self.clock.date_text();
        let date_style = self.date_text_style();
        let username_text = self.username_text.as_deref();
        let username_style = self.username_text_style();
        let placeholder_style = self.placeholder_text_style();
        let status_text = if input_visible && status_mode_external {
            self.status_text()
        } else {
            (!input_visible).then(|| self.status_text()).flatten()
        };
        let hidden_reveal_hint = self.hidden_reveal_hint();
        let status_style = if hidden_reveal_hint.is_some() && status_text.is_none() {
            self.reveal_text_style()
        } else {
            self.status_text_style()
        };
        let weather = self.weather.as_ref();
        let weather_temperature_style = self.weather_temperature_text_style();
        let weather_location_style = self.weather_location_text_style();

        self.text_layout_cache
            .borrow_mut()
            .scene_text_blocks(SceneTextInputs {
                clock_style_mode: self.theme.clock_style,
                clock_text: self.theme.clock_enabled.then_some(clock_text),
                clock_secondary_text: self
                    .theme
                    .clock_enabled
                    .then_some(())
                    .and(clock_secondary_text),
                clock_style,
                clock_meridiem_text: self
                    .theme
                    .clock_enabled
                    .then_some(())
                    .and(clock_meridiem_text),
                clock_meridiem_style,
                clock_meridiem_x,
                clock_meridiem_y,
                date_text: self.theme.date_enabled.then_some(date_text),
                date_style,
                username_text: identity_visible
                    .then_some(())
                    .and(self.theme.username_enabled.then_some(()))
                    .and(username_text),
                username_style,
                placeholder_text: input_visible.then_some(()).and(
                    self.theme
                        .placeholder_enabled
                        .then_some(self.hint_text.as_str()),
                ),
                placeholder_style,
                status_text: if input_visible {
                    if status_mode_external {
                        self.theme
                            .status_enabled
                            .then_some(())
                            .and(status_text.as_deref())
                    } else {
                        None
                    }
                } else {
                    hidden_reveal_hint
                },
                status_style,
                weather_temperature_text: if self.theme.weather_enabled
                    && self.theme.weather_temperature_enabled
                {
                    weather.map(|weather| weather.temperature_text.as_str())
                } else {
                    None
                },
                weather_temperature_style,
                weather_location_text: if self.theme.weather_enabled
                    && self.theme.weather_location_enabled
                {
                    weather.map(|weather| weather.location.as_str())
                } else {
                    None
                },
                weather_location_style,
                weather_icon: if self.theme.weather_enabled && self.theme.weather_icon_enabled {
                    weather.map(|weather| weather.icon)
                } else {
                    None
                },
                weather_icon_size: self.theme.weather_icon_size,
                weather_icon_opacity: self.theme.weather_icon_opacity,
                metrics,
            })
    }
}
