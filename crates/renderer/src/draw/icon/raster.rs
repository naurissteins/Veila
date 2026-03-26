use tiny_skia::{FillRule, Paint, Transform};

use super::{IconRasterKey, ParsedIcon};
use crate::{SoftwareBuffer, draw::skia::color as skia_color};

pub(super) fn rasterize_icon(key: IconRasterKey, parsed: &ParsedIcon) -> Vec<u8> {
    let Some(mut pixmap) = tiny_skia::Pixmap::new(key.width, key.height) else {
        return Vec::new();
    };
    let inset = key.padding.max(0) as f32;
    let target_width = (key.width as f32 - inset * 2.0).max(1.0);
    let target_height = (key.height as f32 - inset * 2.0).max(1.0);
    let scale = (target_width / parsed.viewbox.width).min(target_height / parsed.viewbox.height);
    let icon_width = parsed.viewbox.width * scale;
    let icon_height = parsed.viewbox.height * scale;
    let translate_x = ((key.width as f32 - icon_width) / 2.0).max(0.0);
    let translate_y = ((key.height as f32 - icon_height) / 2.0).max(0.0);
    let transform = Transform::from_scale(scale, scale).post_translate(translate_x, translate_y);
    let mut paint = Paint::default();
    paint.set_color(skia_color(key.color));
    paint.anti_alias = true;
    pixmap.fill_path(&parsed.path, &paint, FillRule::Winding, transform, None);
    pixmap.take()
}

pub(super) fn blend_icon_raster(
    buffer: &mut SoftwareBuffer,
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    if pixels.is_empty() || width == 0 || height == 0 {
        return;
    }

    let target_width = buffer.size().width as i32;
    let target_height = buffer.size().height as i32;
    let overlay_width = width as i32;
    let overlay_height = height as i32;

    let left = origin_x.clamp(0, target_width);
    let top = origin_y.clamp(0, target_height);
    let right = (origin_x + overlay_width).clamp(0, target_width);
    let bottom = (origin_y + overlay_height).clamp(0, target_height);

    if left >= right || top >= bottom {
        return;
    }

    let overlay_stride = width as usize * 4;
    let buffer_stride = buffer.size().width as usize * 4;
    let target_pixels = buffer.pixels_mut();

    for y in top..bottom {
        let overlay_y = (y - origin_y) as usize;
        let buffer_y = y as usize;
        for x in left..right {
            let overlay_x = (x - origin_x) as usize;
            let buffer_x = x as usize;
            let src_offset = overlay_y * overlay_stride + overlay_x * 4;
            let dst_offset = buffer_y * buffer_stride + buffer_x * 4;
            blend_pixel(
                &mut target_pixels[dst_offset..dst_offset + 4],
                &pixels[src_offset..src_offset + 4],
            );
        }
    }
}

fn blend_pixel(dst: &mut [u8], src: &[u8]) {
    let src_alpha = src[3] as u16;
    if src_alpha == 0 {
        return;
    }

    if src_alpha == u16::from(u8::MAX) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = src[3];
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    dst[0] = blend_component(dst[0], src[2], inverse_alpha);
    dst[1] = blend_component(dst[1], src[1], inverse_alpha);
    dst[2] = blend_component(dst[2], src[0], inverse_alpha);
    dst[3] = blend_component(dst[3], src[3], inverse_alpha);
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}
