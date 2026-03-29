use image::{RgbaImage, imageops};
use tiny_skia::{FillRule, Paint, PathBuilder, Stroke, Transform};

use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

use super::{
    shape::fill_rect,
    skia::{color as skia_color, draw_overlay},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BackdropLayerMode {
    Solid,
    #[default]
    Blur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackdropLayerStyle {
    pub mode: BackdropLayerMode,
    pub color: ClearColor,
    pub blur_radius: u8,
    pub radius: i32,
    pub border_color: Option<ClearColor>,
    pub border_width: i32,
}

impl BackdropLayerStyle {
    pub const fn new(
        mode: BackdropLayerMode,
        color: ClearColor,
        blur_radius: u8,
        radius: i32,
        border_color: Option<ClearColor>,
        border_width: i32,
    ) -> Self {
        Self {
            mode,
            color,
            blur_radius,
            radius,
            border_color,
            border_width,
        }
    }
}

pub fn draw_backdrop_layer(buffer: &mut SoftwareBuffer, rect: Rect, style: BackdropLayerStyle) {
    if rect.is_empty() {
        return;
    }

    let clipped = clip_rect(rect, buffer.size());
    if clipped.is_empty() {
        return;
    }

    match style.mode {
        BackdropLayerMode::Solid => {
            fill_layer_shape(buffer, clipped, style.color, style.radius);
        }
        BackdropLayerMode::Blur => {
            blur_region(buffer, clipped, style.blur_radius, style.radius);
            if style.color.alpha > 0 {
                fill_layer_shape(buffer, clipped, style.color, style.radius);
            }
        }
    }

    if let Some(border_color) = style.border_color.filter(|color| color.alpha > 0)
        && style.border_width > 0
    {
        stroke_layer_shape(
            buffer,
            clipped,
            border_color,
            style.radius,
            style.border_width,
        );
    }
}

fn blur_region(buffer: &mut SoftwareBuffer, rect: Rect, blur_radius: u8, radius: i32) {
    let width = rect.width.max(0) as u32;
    let height = rect.height.max(0) as u32;
    if width == 0 || height == 0 {
        return;
    }

    let rgba = extract_rgba_region(buffer, rect);
    let Some(region) = RgbaImage::from_raw(width, height, rgba) else {
        return;
    };
    let original = region.clone();
    let blurred = if blur_radius == 0 {
        region
    } else {
        imageops::blur(&region, f32::from(blur_radius.min(24)))
    };
    if radius <= 0 {
        write_rgba_region(buffer, rect, &blurred);
        return;
    }

    let rgba = apply_rounded_mask(original, blurred, radius);
    write_rgba_region(buffer, rect, &rgba);
}

fn fill_layer_shape(buffer: &mut SoftwareBuffer, rect: Rect, color: ClearColor, radius: i32) {
    if radius <= 0 {
        fill_rect(buffer, rect, color);
        return;
    }

    draw_overlay(
        buffer,
        rect.x,
        rect.y,
        rect.width.max(1) as u32,
        rect.height.max(1) as u32,
        |overlay| {
            let Some(path) = rounded_rect_path(rect.width, rect.height, radius) else {
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
        },
    );
}

fn stroke_layer_shape(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    color: ClearColor,
    radius: i32,
    width: i32,
) {
    draw_overlay(
        buffer,
        rect.x,
        rect.y,
        rect.width.max(1) as u32,
        rect.height.max(1) as u32,
        |overlay| {
            let Some(path) = rounded_rect_path(rect.width, rect.height, radius) else {
                return;
            };

            let mut paint = Paint::default();
            paint.set_color(skia_color(color));
            paint.anti_alias = true;

            let stroke = Stroke {
                width: width.max(1) as f32,
                ..Stroke::default()
            };
            overlay.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        },
    );
}

fn clip_rect(rect: Rect, size: FrameSize) -> Rect {
    let left = rect.x.clamp(0, size.width as i32);
    let top = rect.y.clamp(0, size.height as i32);
    let right = (rect.x + rect.width).clamp(0, size.width as i32);
    let bottom = (rect.y + rect.height).clamp(0, size.height as i32);

    Rect::new(left, top, right - left, bottom - top)
}

fn extract_rgba_region(buffer: &SoftwareBuffer, rect: Rect) -> Vec<u8> {
    let stride = buffer.size().width as usize * 4;
    let mut rgba = Vec::with_capacity(rect.width as usize * rect.height as usize * 4);

    for y in rect.y as usize..(rect.y + rect.height) as usize {
        let row_start = y * stride;
        for x in rect.x as usize..(rect.x + rect.width) as usize {
            let offset = row_start + x * 4;
            let pixel = &buffer.pixels()[offset..offset + 4];
            let blue = pixel[0];
            let green = pixel[1];
            let red = pixel[2];
            let alpha = pixel[3];

            if alpha == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                rgba.extend_from_slice(&[
                    unpremultiply_channel(red, alpha),
                    unpremultiply_channel(green, alpha),
                    unpremultiply_channel(blue, alpha),
                    alpha,
                ]);
            }
        }
    }

    rgba
}

fn write_rgba_region(buffer: &mut SoftwareBuffer, rect: Rect, image: &RgbaImage) {
    let stride = buffer.size().width as usize * 4;
    let pixels = buffer.pixels_mut();

    for (row_index, y) in (rect.y as usize..(rect.y + rect.height) as usize).enumerate() {
        let row_start = y * stride;
        for (column_index, x) in (rect.x as usize..(rect.x + rect.width) as usize).enumerate() {
            let dst = row_start + x * 4;
            let src = image.get_pixel(column_index as u32, row_index as u32).0;
            let alpha = src[3];

            pixels[dst] = premultiply_channel(src[2], alpha);
            pixels[dst + 1] = premultiply_channel(src[1], alpha);
            pixels[dst + 2] = premultiply_channel(src[0], alpha);
            pixels[dst + 3] = alpha;
        }
    }
}

fn apply_rounded_mask(original: RgbaImage, blurred: RgbaImage, radius: i32) -> RgbaImage {
    let width = blurred.width();
    let height = blurred.height();
    let Some(mask) = rounded_mask(width, height, radius) else {
        return blurred;
    };
    let mut output = original;

    for y in 0..height {
        for x in 0..width {
            let mask_alpha = u16::from(mask.get_pixel(x, y).0[3]);
            if mask_alpha == 0 {
                continue;
            }
            if mask_alpha == 255 {
                output.put_pixel(x, y, *blurred.get_pixel(x, y));
                continue;
            }

            let src = blurred.get_pixel(x, y).0;
            let dst = output.get_pixel(x, y).0;
            let mut blended = [0u8; 4];
            for index in 0..4 {
                blended[index] = (((u16::from(src[index]) * mask_alpha)
                    + (u16::from(dst[index]) * (255 - mask_alpha))
                    + 127)
                    / 255) as u8;
            }
            output.put_pixel(x, y, image::Rgba(blended));
        }
    }

    output
}

fn rounded_mask(width: u32, height: u32, radius: i32) -> Option<RgbaImage> {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let path = rounded_rect_path(width as i32, height as i32, radius)?;
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

const fn premultiply_channel(channel: u8, alpha: u8) -> u8 {
    ((channel as u16 * alpha as u16 + 127) / 255) as u8
}

fn unpremultiply_channel(channel: u8, alpha: u8) -> u8 {
    (((u16::from(channel) * 255) + (u16::from(alpha) / 2)) / u16::from(alpha)) as u8
}

#[cfg(test)]
mod tests {
    use super::{BackdropLayerMode, BackdropLayerStyle, draw_backdrop_layer};
    use crate::{
        ClearColor, FrameSize, SoftwareBuffer,
        shape::{Rect, fill_rect},
    };

    #[test]
    fn draws_solid_backdrop_layer() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(4, 4), ClearColor::opaque(0, 0, 0)).unwrap();

        draw_backdrop_layer(
            &mut buffer,
            Rect::new(1, 0, 2, 4),
            BackdropLayerStyle::new(
                BackdropLayerMode::Solid,
                ClearColor::rgba(255, 255, 255, 64),
                0,
                0,
                None,
                0,
            ),
        );

        assert!(buffer.pixels()[7] > 0);
    }

    #[test]
    fn blur_backdrop_layer_changes_region_pixels() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(4, 4), ClearColor::opaque(0, 0, 0)).unwrap();
        fill_rect(
            &mut buffer,
            Rect::new(0, 0, 2, 4),
            ClearColor::opaque(255, 255, 255),
        );

        let before = buffer.pixels().to_vec();
        draw_backdrop_layer(
            &mut buffer,
            Rect::new(0, 0, 4, 4),
            BackdropLayerStyle::new(
                BackdropLayerMode::Blur,
                ClearColor::rgba(8, 10, 14, 0),
                8,
                0,
                None,
                0,
            ),
        );

        assert_ne!(buffer.pixels(), before.as_slice());
    }

    #[test]
    fn rounded_blur_layer_preserves_corner_pixels() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(8, 8), ClearColor::opaque(0, 0, 0)).unwrap();
        fill_rect(
            &mut buffer,
            Rect::new(0, 0, 8, 8),
            ClearColor::opaque(255, 255, 255),
        );

        let before_corner = buffer.pixels()[..4].to_vec();
        draw_backdrop_layer(
            &mut buffer,
            Rect::new(0, 0, 8, 8),
            BackdropLayerStyle::new(
                BackdropLayerMode::Blur,
                ClearColor::rgba(8, 10, 14, 0),
                8,
                3,
                None,
                0,
            ),
        );

        assert_eq!(&buffer.pixels()[..4], before_corner.as_slice());
    }

    #[test]
    fn draws_rounded_layer_border() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(8, 8), ClearColor::opaque(0, 0, 0)).unwrap();

        draw_backdrop_layer(
            &mut buffer,
            Rect::new(1, 1, 6, 6),
            BackdropLayerStyle::new(
                BackdropLayerMode::Solid,
                ClearColor::rgba(8, 10, 14, 0),
                0,
                2,
                Some(ClearColor::opaque(255, 255, 255)),
                1,
            ),
        );

        assert!(buffer.pixels()[4 * (8 + 3) + 2] > 0);
    }
}
