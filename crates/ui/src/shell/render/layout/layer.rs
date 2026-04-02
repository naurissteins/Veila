use veila_common::{LayerAlignment, LayerVerticalAlignment};
use veila_renderer::shape::Rect;

use super::types::LayerPlacement;

pub fn layer_rect(frame_width: i32, frame_height: i32, placement: LayerPlacement) -> Rect {
    let left_padding = placement
        .left_padding
        .unwrap_or(0)
        .clamp(0, frame_width.max(0));
    let right_padding = placement
        .right_padding
        .unwrap_or(0)
        .clamp(0, frame_width.max(0));
    let top_padding = placement
        .top_padding
        .unwrap_or(0)
        .clamp(0, frame_height.max(0));
    let bottom_padding = placement
        .bottom_padding
        .unwrap_or(0)
        .clamp(0, frame_height.max(0));
    let safe_left = left_padding;
    let safe_right = (frame_width - right_padding).max(safe_left + 1);
    let safe_width = (safe_right - safe_left).max(1);
    let safe_top = top_padding.min(frame_height.saturating_sub(1));
    let safe_bottom = (frame_height - bottom_padding).max(safe_top + 1);
    let safe_height = (safe_bottom - safe_top).max(1);
    let width = if placement.full_width {
        safe_width
    } else {
        placement
            .width
            .unwrap_or((frame_width as f32 * 0.36) as i32)
            .clamp(1, safe_width)
    };
    let height = if placement.full_height {
        safe_height
    } else {
        placement
            .height
            .unwrap_or(safe_height)
            .clamp(1, safe_height)
    };
    let offset_x = placement.offset_x.unwrap_or(0);
    let offset_y = placement.offset_y.unwrap_or(0);
    let unclamped_x = match placement.alignment {
        LayerAlignment::Left => safe_left + offset_x,
        LayerAlignment::Center => safe_left + (safe_width - width) / 2 + offset_x,
        LayerAlignment::Right => safe_right - width + offset_x,
    };
    let x = unclamped_x.clamp(safe_left - width + 1, safe_right - 1);
    let unclamped_y = match placement.vertical_alignment {
        LayerVerticalAlignment::Top => safe_top,
        LayerVerticalAlignment::Center => safe_top + (safe_height - height) / 2,
        LayerVerticalAlignment::Bottom => safe_bottom - height,
    } + offset_y;
    let y = unclamped_y.clamp(safe_top, safe_bottom - height);

    Rect::new(x, y, width, height)
}

pub fn layer_center_x(frame_width: i32, placement: LayerPlacement) -> i32 {
    let rect = layer_rect(frame_width, 1, placement);
    rect.x + rect.width / 2
}
