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

use veila_common::{LayerAlignment, LayerMode, LayerStyle};
use veila_renderer::{
    SoftwareBuffer,
    layer::{
        BackdropLayerAlignment, BackdropLayerMode, BackdropLayerShape, BackdropLayerStyle,
        draw_backdrop_layer,
    },
    shape::Rect,
};

use self::{
    cache::SceneTextInputs,
    layout::{
        AnchorOffsets, AuthGroupHeights, FooterHeights, InputPlacement, LayerPlacement,
        RoleAnchorInput, RoleAnchors, SceneMetrics, layer_center_x, layer_rect,
        role_anchors_with_groups,
    },
    model::{AuthGroup, LayoutRole, SceneModel, SceneTextBlocks, SceneWidget},
};
use super::ShellState;

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
    fn scene_layout(&self, size: veila_renderer::FrameSize) -> SceneLayout {
        let layer_placement = LayerPlacement {
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
        };
        let layer_center_x = (self.theme.layer_enabled && self.theme.input_center_in_layer)
            .then(|| layer_center_x(size.width as i32, layer_placement));
        let metrics = SceneMetrics::from_frame_with_input_placement(
            size.width as i32,
            size.height as i32,
            self.theme.input_width,
            self.theme.input_height,
            self.theme.avatar_size,
            InputPlacement {
                alignment: self.theme.input_alignment,
                center_in_layer: self.theme.input_center_in_layer,
                layer_center_x,
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
        let footer_render_height =
            model.total_height_for_role(LayoutRole::Footer, metrics, &self.status);
        let footer_clearance_height =
            self.footer_clearance_height(&model, size.width as i32, metrics);
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
                render: footer_render_height,
                clearance: footer_clearance_height,
            },
            input_alignment: self.theme.input_alignment,
            offsets: AnchorOffsets {
                auth_stack: self.theme.auth_stack_offset,
                input_vertical_padding: self.theme.input_vertical_padding,
                input_offset_y: self.theme.input_offset_y,
                header_top: self.theme.header_top_offset,
                identity_gap: self.theme.identity_gap,
                center_stack_style: self.theme.center_stack_style,
                clock_alignment: self.theme.clock_alignment,
                clock_offset_y: self.theme.clock_offset_y,
                weather_bottom_padding: self.theme.weather_bottom_padding,
            },
        });

        SceneLayout {
            metrics,
            model,
            anchors,
        }
    }

    fn footer_clearance_height(
        &self,
        model: &SceneModel,
        frame_width: i32,
        metrics: SceneMetrics,
    ) -> i32 {
        let auth_left = metrics.auth_center_x - metrics.content_width as i32 / 2;
        let auth_right = metrics.auth_center_x + metrics.content_width as i32 / 2;

        model
            .sections_for_role(LayoutRole::Footer)
            .filter_map(|section| match &section.widget {
                SceneWidget::Weather(weather) => {
                    let widget_left = match weather.alignment {
                        veila_common::WeatherAlignment::Left => {
                            weather.horizontal_padding + weather.left_offset
                        }
                        veila_common::WeatherAlignment::Right => {
                            frame_width - weather.horizontal_padding - weather.width()
                                + weather.left_offset
                        }
                    };
                    let widget_right = widget_left + weather.width();

                    horizontal_ranges_overlap(auth_left, auth_right, widget_left, widget_right)
                        .then_some(section.height(metrics, &self.status) + section.gap_after)
                }
                _ => Some(section.height(metrics, &self.status) + section.gap_after),
            })
            .sum()
    }

    pub fn render_backdrop_layer(&self, buffer: &mut SoftwareBuffer) {
        if !self.theme.layer_enabled {
            return;
        }

        let Some(rect) = self.backdrop_layer_rect(buffer.size()) else {
            return;
        };

        let mode = match self.theme.layer_mode {
            LayerMode::Solid => BackdropLayerMode::Solid,
            LayerMode::Blur => BackdropLayerMode::Blur,
        };
        let alignment = match self.theme.layer_alignment {
            LayerAlignment::Left => BackdropLayerAlignment::Left,
            LayerAlignment::Center => BackdropLayerAlignment::Center,
            LayerAlignment::Right => BackdropLayerAlignment::Right,
        };
        let shape = match self.theme.layer_style {
            LayerStyle::Panel => BackdropLayerShape::Panel,
            LayerStyle::Diagonal => BackdropLayerShape::Diagonal(alignment),
        };

        draw_backdrop_layer(
            buffer,
            rect,
            BackdropLayerStyle::new(
                mode,
                shape,
                self.theme.layer_color,
                self.theme.layer_blur_radius,
                self.theme.layer_radius,
                self.theme.layer_border_color,
                self.theme.layer_border_width,
            ),
        );
    }

    fn backdrop_layer_rect(&self, size: veila_renderer::FrameSize) -> Option<Rect> {
        Some(layer_rect(
            size.width as i32,
            size.height as i32,
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
        ))
    }

    fn scene_text_blocks(&self, metrics: SceneMetrics) -> SceneTextBlocks {
        let clock_text = self.clock.primary_text(self.theme.clock_style);
        let clock_secondary_text = self.clock.secondary_text(self.theme.clock_style);
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
}

fn horizontal_ranges_overlap(left_a: i32, right_a: i32, left_b: i32, right_b: i32) -> bool {
    left_a < right_b && left_b < right_a
}
