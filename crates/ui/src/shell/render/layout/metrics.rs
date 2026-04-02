use veila_common::InputAlignment;
use veila_renderer::shape::Rect;

use super::{SceneMetrics, types::InputPlacement};

impl SceneMetrics {
    #[cfg(test)]
    pub fn from_frame(
        width: i32,
        height: i32,
        configured_input_width: Option<i32>,
        configured_input_height: Option<i32>,
        configured_avatar_size: Option<i32>,
        input_alignment: InputAlignment,
    ) -> Self {
        Self::from_frame_with_input_placement(
            width,
            height,
            configured_input_width,
            configured_input_height,
            configured_avatar_size,
            InputPlacement {
                alignment: input_alignment,
                center_in_layer: false,
                layer_center_x: None,
                horizontal_padding: None,
                offset_x: None,
            },
        )
    }

    pub fn from_frame_with_input_placement(
        width: i32,
        height: i32,
        configured_input_width: Option<i32>,
        configured_input_height: Option<i32>,
        configured_avatar_size: Option<i32>,
        input_placement: InputPlacement,
    ) -> Self {
        let scene_width = ((width as f32) * 0.34) as i32;
        let input_width = configured_input_width
            .unwrap_or_else(|| (((scene_width as f32) * 0.7) as i32).clamp(220, 320))
            .clamp(180, 560);
        let input_height = configured_input_height
            .unwrap_or_else(|| (((height as f32) * 0.072) as i32).clamp(48, 58))
            .clamp(40, 96);
        let avatar_size = configured_avatar_size
            .unwrap_or_else(|| (width.min(height) / 7).clamp(84, 108))
            .clamp(56, 160);
        let horizontal_padding = input_placement
            .horizontal_padding
            .unwrap_or_else(|| super::horizontal_auth_padding(width));
        let base_auth_center_x = if input_placement.center_in_layer {
            input_placement.layer_center_x.unwrap_or_else(|| {
                auth_center_x(
                    width,
                    input_width,
                    horizontal_padding,
                    input_placement.alignment,
                )
            })
        } else {
            auth_center_x(
                width,
                input_width,
                horizontal_padding,
                input_placement.alignment,
            )
        };
        let auth_center_x = apply_auth_offset_x(
            base_auth_center_x,
            width,
            input_width,
            input_placement.offset_x,
        );

        Self {
            center_x: width / 2,
            auth_center_x,
            content_width: (input_width + 72).max(220) as u32,
            clock_width: (input_width + 140).max(280) as u32,
            input_width,
            input_height,
            avatar_size,
        }
    }

    pub fn input_rect(self, y: i32) -> Rect {
        Rect::new(
            self.auth_center_x - self.input_width / 2,
            y,
            self.input_width,
            self.input_height,
        )
    }
}

fn auth_center_x(
    frame_width: i32,
    input_width: i32,
    horizontal_padding: i32,
    input_alignment: InputAlignment,
) -> i32 {
    let centered = frame_width / 2;
    let left = (horizontal_padding + input_width / 2).clamp(input_width / 2, frame_width);
    let right =
        (frame_width - horizontal_padding - input_width / 2).clamp(input_width / 2, frame_width);
    match input_alignment {
        InputAlignment::TopLeft | InputAlignment::CenterLeft | InputAlignment::BottomLeft => left,
        InputAlignment::TopRight | InputAlignment::CenterRight | InputAlignment::BottomRight => {
            right
        }
        InputAlignment::TopCenter | InputAlignment::CenterCenter | InputAlignment::BottomCenter => {
            centered
        }
    }
}

fn apply_auth_offset_x(
    auth_center_x: i32,
    frame_width: i32,
    input_width: i32,
    input_offset_x: Option<i32>,
) -> i32 {
    let min_x = input_width / 2;
    let max_x = (frame_width - input_width / 2).max(min_x);
    (auth_center_x + input_offset_x.unwrap_or(0)).clamp(min_x, max_x)
}
