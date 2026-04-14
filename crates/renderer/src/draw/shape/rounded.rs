use tiny_skia::{FillRule, Paint, Path, PathBuilder, Stroke, Transform};

use crate::{ClearColor, SoftwareBuffer};

use super::{CircleStyle, PillStyle, Rect};
use crate::draw::skia::{color as skia_color, draw_overlay};

/// Draws a modern pill surface using tiny-skia.
pub fn draw_pill(buffer: &mut SoftwareBuffer, rect: Rect, style: PillStyle) {
    if rect.is_empty() {
        return;
    }

    let radius = resolved_corner_radius(rect, style.radius);
    let overlay_padding = style
        .shadow
        .map(|shadow| shadow.offset_x.abs().max(shadow.offset_y.abs()))
        .unwrap_or(0)
        + style
            .border
            .map(|border| border.thickness.max(0))
            .unwrap_or(0)
        + 2;
    let overlay_origin_x = rect.x - overlay_padding;
    let overlay_origin_y = rect.y - overlay_padding;
    let overlay_width = (rect.width + overlay_padding * 2).max(1) as u32;
    let overlay_height = (rect.height + overlay_padding * 2).max(1) as u32;

    draw_overlay(
        buffer,
        overlay_origin_x,
        overlay_origin_y,
        overlay_width,
        overlay_height,
        |overlay| {
            let offset_x = overlay_padding as f32;
            let offset_y = overlay_padding as f32;

            if let Some(shadow) = style.shadow {
                fill_rounded_rect_path(
                    overlay,
                    rect.width,
                    rect.height,
                    radius,
                    shadow.color,
                    offset_x + shadow.offset_x as f32,
                    offset_y + shadow.offset_y as f32,
                );
            }

            fill_rounded_rect_path(
                overlay,
                rect.width,
                rect.height,
                radius,
                style.fill,
                offset_x,
                offset_y,
            );

            if let Some(border) = style.border {
                stroke_rounded_rect_path(
                    overlay,
                    rect,
                    radius,
                    border.thickness.max(1),
                    border.color,
                    offset_x,
                    offset_y,
                );
            }
        },
    );
}

/// Draws a filled circle with optional border and shadow using tiny-skia.
pub fn draw_circle(
    buffer: &mut SoftwareBuffer,
    center_x: i32,
    center_y: i32,
    radius: i32,
    style: CircleStyle,
) {
    if radius <= 0 {
        return;
    }

    let diameter = radius * 2;
    let bounds = Rect::new(center_x - radius, center_y - radius, diameter, diameter);
    let overlay_padding = style
        .shadow
        .map(|shadow| shadow.offset_x.abs().max(shadow.offset_y.abs()))
        .unwrap_or(0)
        + style
            .border
            .map(|border| border.thickness.max(0))
            .unwrap_or(0)
        + 2;
    let overlay_origin_x = bounds.x - overlay_padding;
    let overlay_origin_y = bounds.y - overlay_padding;
    let overlay_size = (diameter + overlay_padding * 2).max(1) as u32;

    draw_overlay(
        buffer,
        overlay_origin_x,
        overlay_origin_y,
        overlay_size,
        overlay_size,
        |overlay| {
            let center_x = (overlay_padding + radius) as f32;
            let center_y = (overlay_padding + radius) as f32;

            if let Some(shadow) = style.shadow {
                fill_circle_path(
                    overlay,
                    center_x + shadow.offset_x as f32,
                    center_y + shadow.offset_y as f32,
                    radius as f32,
                    shadow.color,
                );
            }

            if let Some(border) = style.border {
                let border_thickness = border.thickness.max(1).min(radius);
                let inner_radius = (radius - border_thickness).max(0) as f32;
                if inner_radius > 0.0 {
                    fill_circle_path(overlay, center_x, center_y, inner_radius, style.fill);
                }
                stroke_circle_path(
                    overlay,
                    center_x,
                    center_y,
                    radius as f32 - border_thickness as f32 / 2.0,
                    border_thickness as f32,
                    border.color,
                );
            } else {
                fill_circle_path(overlay, center_x, center_y, radius as f32, style.fill);
            }
        },
    );
}

fn resolved_corner_radius(rect: Rect, radius: i32) -> i32 {
    let max_radius = (rect.width.min(rect.height) / 2).max(0);
    if radius == i32::MAX {
        max_radius
    } else {
        radius.clamp(0, max_radius)
    }
}

fn fill_rounded_rect_path(
    overlay: &mut tiny_skia::Pixmap,
    width: i32,
    height: i32,
    radius: i32,
    color: ClearColor,
    offset_x: f32,
    offset_y: f32,
) {
    if width <= 0 || height <= 0 {
        return;
    }

    let Some(path) = rounded_rect_path(
        width as f32,
        height as f32,
        radius as f32,
        offset_x,
        offset_y,
    ) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;
    overlay.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn stroke_rounded_rect_path(
    overlay: &mut tiny_skia::Pixmap,
    rect: Rect,
    radius: i32,
    stroke_width: i32,
    color: ClearColor,
    offset_x: f32,
    offset_y: f32,
) {
    if rect.is_empty() || stroke_width <= 0 {
        return;
    }

    let inset = stroke_width as f32 / 2.0;
    let Some(path) = rounded_rect_path(
        rect.width as f32 - stroke_width as f32,
        rect.height as f32 - stroke_width as f32,
        radius as f32 - inset,
        offset_x + inset,
        offset_y + inset,
    ) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;

    let stroke = Stroke {
        width: stroke_width as f32,
        ..Stroke::default()
    };
    overlay.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

fn rounded_rect_path(
    width: f32,
    height: f32,
    radius: f32,
    offset_x: f32,
    offset_y: f32,
) -> Option<Path> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let right = offset_x + width;
    let bottom = offset_y + height;
    let radius = radius.max(0.0).min(width.min(height) / 2.0);
    let mut builder = PathBuilder::new();

    if radius <= 0.0 {
        builder.move_to(offset_x, offset_y);
        builder.line_to(right, offset_y);
        builder.line_to(right, bottom);
        builder.line_to(offset_x, bottom);
    } else {
        builder.move_to(offset_x + radius, offset_y);
        builder.line_to(right - radius, offset_y);
        builder.quad_to(right, offset_y, right, offset_y + radius);
        builder.line_to(right, bottom - radius);
        builder.quad_to(right, bottom, right - radius, bottom);
        builder.line_to(offset_x + radius, bottom);
        builder.quad_to(offset_x, bottom, offset_x, bottom - radius);
        builder.line_to(offset_x, offset_y + radius);
        builder.quad_to(offset_x, offset_y, offset_x + radius, offset_y);
    }

    builder.close();
    builder.finish()
}

fn fill_circle_path(
    overlay: &mut tiny_skia::Pixmap,
    center_x: f32,
    center_y: f32,
    radius: f32,
    color: ClearColor,
) {
    let Some(path) = PathBuilder::from_circle(center_x, center_y, radius.max(1.0)) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;
    overlay.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn stroke_circle_path(
    overlay: &mut tiny_skia::Pixmap,
    center_x: f32,
    center_y: f32,
    radius: f32,
    width: f32,
    color: ClearColor,
) {
    if radius <= 0.0 || width <= 0.0 {
        return;
    }

    let Some(path) = PathBuilder::from_circle(center_x, center_y, radius) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;

    let stroke = Stroke {
        width,
        ..Stroke::default()
    };
    overlay.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}
