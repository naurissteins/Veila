use resvg::usvg;
use tiny_skia::{FillRule, Paint, Transform};

use super::{IconRasterKey, IconRasterSource, ParsedIcon};
use crate::{PixelBuffer, draw::skia::color as skia_color};

pub(super) fn rasterize_icon(key: IconRasterKey, source: IconRasterSource) -> Vec<u8> {
    match source {
        IconRasterSource::Parsed(parsed) => rasterize_parsed_icon(key, parsed),
        IconRasterSource::Svg(svg) => rasterize_svg_icon(key, svg),
    }
}

fn rasterize_parsed_icon(key: IconRasterKey, parsed: &ParsedIcon) -> Vec<u8> {
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
    let translate_y = svg_translate_y(key, icon_height);
    let transform = Transform::from_scale(scale, scale).post_translate(translate_x, translate_y);
    let mut paint = Paint::default();
    paint.set_color(skia_color(key.color));
    paint.anti_alias = true;
    pixmap.fill_path(&parsed.path, &paint, FillRule::Winding, transform, None);
    pixmap.take()
}

fn rasterize_svg_icon(key: IconRasterKey, svg: &[u8]) -> Vec<u8> {
    let Some(mut pixmap) = tiny_skia::Pixmap::new(key.width, key.height) else {
        return Vec::new();
    };
    let options = usvg::Options::default();
    let Ok(tree) = usvg::Tree::from_data(svg, &options) else {
        return Vec::new();
    };
    resvg::render(&tree, svg_transform(key, &tree), &mut pixmap.as_mut());
    let mut pixels = pixmap.take();
    if matches!(
        key.icon,
        super::AssetIcon::Battery(_) | super::AssetIcon::CapsLock | super::AssetIcon::Power(_)
    ) {
        recolor_svg_pixels(&mut pixels, key.color);
    }
    scale_svg_alpha(&mut pixels, key.color.alpha);
    pixels
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AlphaBounds {
    pub(super) left: u32,
    pub(super) top: u32,
    pub(super) right: u32,
    pub(super) bottom: u32,
}

impl AlphaBounds {
    pub(super) const fn width(self) -> u32 {
        self.right.saturating_sub(self.left)
    }

    pub(super) const fn height(self) -> u32 {
        self.bottom.saturating_sub(self.top)
    }
}

fn svg_transform(key: IconRasterKey, tree: &usvg::Tree) -> Transform {
    let bounds = tree.root().abs_layer_bounding_box();
    let inset = key.padding.max(0) as f32;
    let target_width = (key.width as f32 - inset * 2.0).max(1.0);
    let target_height = (key.height as f32 - inset * 2.0).max(1.0);
    let scale = (target_width / bounds.width()).min(target_height / bounds.height());
    let icon_width = bounds.width() * scale;
    let icon_height = bounds.height() * scale;
    let translate_x = ((key.width as f32 - icon_width) / 2.0).max(0.0);
    let translate_y = ((key.height as f32 - icon_height) / 2.0).max(0.0);

    Transform::from_row(
        scale,
        0.0,
        0.0,
        scale,
        translate_x - bounds.x() * scale,
        translate_y - bounds.y() * scale,
    )
}

pub(super) fn svg_translate_y(key: IconRasterKey, icon_height: f32) -> f32 {
    let inset = key.padding.max(0) as f32;

    match key.icon {
        super::AssetIcon::Weather(_) => ((key.height as f32 - inset) - icon_height).max(inset),
        _ => ((key.height as f32 - icon_height) / 2.0).max(0.0),
    }
}

pub(super) fn visible_alpha_bounds(pixels: &[u8], width: u32, height: u32) -> Option<AlphaBounds> {
    if width == 0 || height == 0 {
        return None;
    }

    let stride = width as usize * 4;
    let mut left = width;
    let mut top = height;
    let mut right = 0;
    let mut bottom = 0;

    for y in 0..height as usize {
        let row_offset = y * stride;
        for x in 0..width as usize {
            let alpha = pixels[row_offset + x * 4 + 3];
            if alpha == 0 {
                continue;
            }

            left = left.min(x as u32);
            top = top.min(y as u32);
            right = right.max(x as u32 + 1);
            bottom = bottom.max(y as u32 + 1);
        }
    }

    (right > left && bottom > top).then_some(AlphaBounds {
        left,
        top,
        right,
        bottom,
    })
}

pub(super) fn scale_svg_alpha(pixels: &mut [u8], alpha: u8) {
    if alpha == u8::MAX {
        return;
    }

    for pixel in pixels.chunks_exact_mut(4) {
        if pixel[3] == 0 {
            continue;
        }

        pixel[0] = ((u16::from(pixel[0]) * u16::from(alpha) + 127) / 255) as u8;
        pixel[1] = ((u16::from(pixel[1]) * u16::from(alpha) + 127) / 255) as u8;
        pixel[2] = ((u16::from(pixel[2]) * u16::from(alpha) + 127) / 255) as u8;
        pixel[3] = ((u16::from(pixel[3]) * u16::from(alpha) + 127) / 255) as u8;
    }
}

fn recolor_svg_pixels(pixels: &mut [u8], color: crate::ClearColor) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha == 0 {
            continue;
        }

        pixel[0] = ((u16::from(color.red) * u16::from(alpha) + 127) / 255) as u8;
        pixel[1] = ((u16::from(color.green) * u16::from(alpha) + 127) / 255) as u8;
        pixel[2] = ((u16::from(color.blue) * u16::from(alpha) + 127) / 255) as u8;
    }
}

pub(super) fn blend_icon_raster(
    buffer: &mut impl PixelBuffer,
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
