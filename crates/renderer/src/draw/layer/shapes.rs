use image::RgbaImage;
use tiny_skia::{FillRule, Paint, PathBuilder};

use super::{BackdropLayerAlignment, BackdropLayerShape, BackdropLayerStyle, LayerSurface};

pub fn layer_path(width: i32, height: i32, style: BackdropLayerStyle) -> Option<tiny_skia::Path> {
    match style.shape {
        BackdropLayerShape::Panel => panel_path(width, height, style.radius, 0.0),
        BackdropLayerShape::Diagonal(alignment) => diagonal_path(width, height, alignment),
    }
}

pub fn layer_mask(surface: LayerSurface, style: BackdropLayerStyle) -> Option<RgbaImage> {
    let width = surface.bounds.width.max(1) as u32;
    let height = surface.bounds.height.max(1) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let path = layer_path(surface.rect.width, surface.rect.height, style)?;
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, FillRule::Winding, surface.transform(), None);
    RgbaImage::from_raw(width, height, pixmap.take())
}

pub(super) fn panel_path(
    width: i32,
    height: i32,
    radius: f32,
    border_width: f32,
) -> Option<tiny_skia::Path> {
    if width <= 0 || height <= 0 {
        return None;
    }

    let width = width as f32;
    let height = height as f32;
    let half_short_side = width.min(height) / 2.0;
    let radius = radius.max(0.0).min(half_short_side);
    let mut builder = PathBuilder::new();
    push_rounded_rect(&mut builder, width, height, radius, 0.0);
    if border_width > 0.0 && border_width < half_short_side {
        push_rounded_rect(&mut builder, width, height, radius, border_width);
    }
    builder.finish()
}

fn push_rounded_rect(builder: &mut PathBuilder, width: f32, height: f32, radius: f32, inset: f32) {
    let right = width - inset;
    let bottom = height - inset;
    let radius = (radius - inset).max(0.0);
    if radius == 0.0 {
        if let Some(rect) = tiny_skia::Rect::from_ltrb(inset, inset, right, bottom) {
            builder.push_rect(rect);
        }
        return;
    }

    // A cubic quarter circle keeps the inset arc concentric with the outer arc.
    let tangent = radius * (1.0 - 0.552_284_8);
    builder.move_to(inset + radius, inset);
    builder.line_to(right - radius, inset);
    builder.cubic_to(
        right - tangent,
        inset,
        right,
        inset + tangent,
        right,
        inset + radius,
    );
    builder.line_to(right, bottom - radius);
    builder.cubic_to(
        right,
        bottom - tangent,
        right - tangent,
        bottom,
        right - radius,
        bottom,
    );
    builder.line_to(inset + radius, bottom);
    builder.cubic_to(
        inset + tangent,
        bottom,
        inset,
        bottom - tangent,
        inset,
        bottom - radius,
    );
    builder.line_to(inset, inset + radius);
    builder.cubic_to(
        inset,
        inset + tangent,
        inset + tangent,
        inset,
        inset + radius,
        inset,
    );
    builder.close();
}

fn diagonal_path(
    width: i32,
    height: i32,
    alignment: BackdropLayerAlignment,
) -> Option<tiny_skia::Path> {
    if width <= 0 || height <= 0 {
        return None;
    }

    let right = width as f32;
    let bottom = height as f32;
    let mut builder = PathBuilder::new();

    match alignment {
        BackdropLayerAlignment::Left => {
            builder.move_to(0.0, 0.0);
            builder.line_to(right, 0.0);
            builder.line_to(0.0, bottom);
        }
        BackdropLayerAlignment::Center => {
            let inset = right * 0.28;
            builder.move_to(inset, 0.0);
            builder.line_to(right, 0.0);
            builder.line_to(right - inset, bottom);
            builder.line_to(0.0, bottom);
        }
        BackdropLayerAlignment::Right => {
            builder.move_to(0.0, 0.0);
            builder.line_to(right, 0.0);
            builder.line_to(right, bottom);
        }
    }

    builder.close();
    builder.finish()
}
