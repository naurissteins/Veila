use std::time::Instant;

use image::RgbaImage;

use crate::{PixelBuffer, blur::blur_rgba, shape::Rect};

use super::shapes::layer_mask;
use super::{BackdropLayerShape, BackdropLayerStyle, LayerSurface};

const SLOW_LAYER_BLUR_MS: u64 = 4;

pub fn blur_region(
    buffer: &mut impl PixelBuffer,
    surface: LayerSurface,
    style: BackdropLayerStyle,
) {
    let rect = surface.bounds;
    let width = rect.width.max(0) as u32;
    let height = rect.height.max(0) as u32;
    if width == 0 || height == 0 {
        return;
    }

    let timing_enabled = tracing::enabled!(tracing::Level::DEBUG);
    let started_at = timing_enabled.then(Instant::now);
    let (rgba, fully_opaque) = extract_rgba_region(buffer, rect);
    let Some(region) = RgbaImage::from_raw(width, height, rgba) else {
        return;
    };
    let blurred = blur_rgba(&region, style.blur_radius, 24);

    if surface.rotate_degrees == 0
        && matches!(style.shape, BackdropLayerShape::Panel)
        && style.radius <= 0
    {
        write_rgba_region(buffer, rect, &blurred, fully_opaque);
        log_blur_timing(started_at, width, height, style, fully_opaque);
        return;
    }

    let rgba = apply_shape_mask(region, blurred, style, surface);
    write_rgba_region(buffer, rect, &rgba, false);
    log_blur_timing(started_at, width, height, style, false);
}

fn extract_rgba_region(buffer: &impl PixelBuffer, rect: Rect) -> (Vec<u8>, bool) {
    let stride = buffer.size().width as usize * 4;
    let mut rgba = Vec::with_capacity(rect.width as usize * rect.height as usize * 4);
    let mut fully_opaque = true;

    for y in rect.y as usize..(rect.y + rect.height) as usize {
        let row_start = y * stride;
        for x in rect.x as usize..(rect.x + rect.width) as usize {
            let offset = row_start + x * 4;
            let pixel = &buffer.pixels()[offset..offset + 4];
            let blue = pixel[0];
            let green = pixel[1];
            let red = pixel[2];
            let alpha = pixel[3];

            if alpha == u8::MAX {
                rgba.extend_from_slice(&[red, green, blue, alpha]);
                continue;
            }

            fully_opaque = false;
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

    (rgba, fully_opaque)
}

fn write_rgba_region(
    buffer: &mut impl PixelBuffer,
    rect: Rect,
    image: &RgbaImage,
    fully_opaque: bool,
) {
    let stride = buffer.size().width as usize * 4;
    let pixels = buffer.pixels_mut();

    for (row_index, y) in (rect.y as usize..(rect.y + rect.height) as usize).enumerate() {
        let row_start = y * stride;
        for (column_index, x) in (rect.x as usize..(rect.x + rect.width) as usize).enumerate() {
            let dst = row_start + x * 4;
            let src = image.get_pixel(column_index as u32, row_index as u32).0;
            let alpha = src[3];

            if fully_opaque || alpha == u8::MAX {
                pixels[dst] = src[2];
                pixels[dst + 1] = src[1];
                pixels[dst + 2] = src[0];
                pixels[dst + 3] = alpha;
                continue;
            }

            pixels[dst] = premultiply_channel(src[2], alpha);
            pixels[dst + 1] = premultiply_channel(src[1], alpha);
            pixels[dst + 2] = premultiply_channel(src[0], alpha);
            pixels[dst + 3] = alpha;
        }
    }
}

fn apply_shape_mask(
    original: RgbaImage,
    blurred: RgbaImage,
    style: BackdropLayerStyle,
    surface: LayerSurface,
) -> RgbaImage {
    let width = surface.bounds.width;
    let height = surface.bounds.height;
    let Some(mask) = layer_mask(surface, style) else {
        return blurred;
    };
    let mut output = original;

    for y in 0..height as u32 {
        for x in 0..width as u32 {
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

const fn premultiply_channel(channel: u8, alpha: u8) -> u8 {
    ((channel as u16 * alpha as u16 + 127) / 255) as u8
}

fn unpremultiply_channel(channel: u8, alpha: u8) -> u8 {
    (((u16::from(channel) * 255) + (u16::from(alpha) / 2)) / u16::from(alpha)) as u8
}

fn log_blur_timing(
    started_at: Option<Instant>,
    width: u32,
    height: u32,
    style: BackdropLayerStyle,
    fully_opaque: bool,
) {
    let Some(started_at) = started_at else {
        return;
    };

    let elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    if elapsed_ms < SLOW_LAYER_BLUR_MS {
        return;
    }

    tracing::debug!(
        width,
        height,
        pixels = u64::from(width) * u64::from(height),
        blur_radius = style.blur_radius,
        shape = ?style.shape,
        radius = style.radius,
        fully_opaque,
        elapsed_ms,
        "layer blur timing"
    );
}
