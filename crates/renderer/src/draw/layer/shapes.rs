use image::RgbaImage;
use tiny_skia::{FillRule, Paint, PathBuilder, Transform};

use super::{BackdropLayerAlignment, BackdropLayerShape, BackdropLayerStyle};

pub fn layer_path(width: i32, height: i32, style: BackdropLayerStyle) -> Option<tiny_skia::Path> {
    match style.shape {
        BackdropLayerShape::Panel => rounded_rect_path(width, height, style.radius),
        BackdropLayerShape::Diagonal(alignment) => diagonal_path(width, height, alignment),
    }
}

pub fn layer_mask(width: u32, height: u32, style: BackdropLayerStyle) -> Option<RgbaImage> {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let path = layer_path(width as i32, height as i32, style)?;
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    RgbaImage::from_raw(width, height, pixmap.take())
}

fn rounded_rect_path(width: i32, height: i32, radius: i32) -> Option<tiny_skia::Path> {
    if width <= 0 || height <= 0 {
        return None;
    }

    let radius = radius.clamp(0, width.min(height) / 2) as f32;
    let right = width as f32;
    let bottom = height as f32;
    let mut builder = PathBuilder::new();

    if radius <= 0.0 {
        builder.move_to(0.0, 0.0);
        builder.line_to(right, 0.0);
        builder.line_to(right, bottom);
        builder.line_to(0.0, bottom);
    } else {
        builder.move_to(radius, 0.0);
        builder.line_to(right - radius, 0.0);
        builder.quad_to(right, 0.0, right, radius);
        builder.line_to(right, bottom - radius);
        builder.quad_to(right, bottom, right - radius, bottom);
        builder.line_to(radius, bottom);
        builder.quad_to(0.0, bottom, 0.0, bottom - radius);
        builder.line_to(0.0, radius);
        builder.quad_to(0.0, 0.0, radius, 0.0);
    }

    builder.close();
    builder.finish()
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
