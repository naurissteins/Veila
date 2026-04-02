use image::{RgbaImage, imageops};

use crate::{SoftwareBuffer, shape::Rect};

use super::shapes::layer_mask;
use super::{BackdropLayerShape, BackdropLayerStyle};

pub fn blur_region(buffer: &mut SoftwareBuffer, rect: Rect, style: BackdropLayerStyle) {
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
    let blurred = if style.blur_radius == 0 {
        region
    } else {
        imageops::blur(&region, f32::from(style.blur_radius.min(24)))
    };

    if matches!(style.shape, BackdropLayerShape::Panel) && style.radius <= 0 {
        write_rgba_region(buffer, rect, &blurred);
        return;
    }

    let rgba = apply_shape_mask(original, blurred, style, rect.width, rect.height);
    write_rgba_region(buffer, rect, &rgba);
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

fn apply_shape_mask(
    original: RgbaImage,
    blurred: RgbaImage,
    style: BackdropLayerStyle,
    width: i32,
    height: i32,
) -> RgbaImage {
    let Some(mask) = layer_mask(width as u32, height as u32, style) else {
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
