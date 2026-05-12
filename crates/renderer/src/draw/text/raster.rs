use std::{cell::RefCell, thread_local};

use cosmic_text::{Buffer, Wrap};

use crate::PixelBuffer;

use super::{
    ClearColor, TextBounds, TextStyle, context::FONT_CONTEXT, font_metrics, modulate_alpha,
    text_attrs, text_color,
};

const TEXT_RASTER_CACHE_LIMIT: usize = 128;

thread_local! {
    static TEXT_RASTER_CACHE: RefCell<Vec<(TextRasterKey, Option<TextRaster>)>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextRasterKey {
    text: String,
    style: TextStyle,
    color: ClearColor,
}

#[derive(Debug, Clone)]
struct TextRaster {
    bounds: TextBounds,
    width: u32,
    pixels: Vec<u8>,
}

pub(super) fn draw_text_lines(
    buffer: &mut impl PixelBuffer,
    x: i32,
    y: i32,
    lines: &[String],
    style: TextStyle,
    color: ClearColor,
) {
    if lines.iter().all(String::is_empty) {
        return;
    }

    let text = lines.join("\n");
    let Some(raster) = cached_text_raster(&text, style, color) else {
        return;
    };

    draw_cached_text_raster(buffer, x, y, &raster);
}

pub(super) fn visible_text_bounds(text: &str, style: TextStyle) -> Option<TextBounds> {
    if text.is_empty() {
        return None;
    }

    if let Some(cached) = TEXT_RASTER_CACHE.with(|cache| {
        let cache = cache.borrow();
        cache
            .iter()
            .find(|(key, _)| key.text == text && key.style == style)
            .and_then(|(_, raster)| raster.as_ref().map(|raster| raster.bounds))
    }) {
        return Some(cached);
    }

    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let super::context::FontContext {
            font_system,
            swash_cache,
        } = &mut *context;
        let mut cosmic_buffer = Buffer::new(font_system, font_metrics(&style));
        cosmic_buffer.set_wrap(font_system, Wrap::None);
        cosmic_buffer.set_size(font_system, None, None);
        let attrs = text_attrs(&style);
        cosmic_buffer.set_text(font_system, text, &attrs, cosmic_text::Shaping::Advanced);
        cosmic_buffer.shape_until_scroll(font_system, true);

        let mut bounds: Option<TextBounds> = None;
        cosmic_buffer.draw(
            font_system,
            swash_cache,
            text_color(style.color),
            |pixel_x, pixel_y, width, height, pixel_color| {
                if pixel_color.a() == 0 || width == 0 || height == 0 {
                    return;
                }

                let next = TextBounds {
                    left: pixel_x,
                    top: pixel_y,
                    right: pixel_x + width as i32,
                    bottom: pixel_y + height as i32,
                };
                bounds = Some(match bounds {
                    Some(current) => TextBounds {
                        left: current.left.min(next.left),
                        top: current.top.min(next.top),
                        right: current.right.max(next.right),
                        bottom: current.bottom.max(next.bottom),
                    },
                    None => next,
                });
            },
        );

        bounds
    })
}

fn cached_text_raster(text: &str, style: TextStyle, color: ClearColor) -> Option<TextRaster> {
    let key = TextRasterKey {
        text: text.to_owned(),
        style,
        color,
    };

    TEXT_RASTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((_, raster)) = cache.iter().find(|(candidate, _)| candidate == &key) {
            return raster.clone();
        }

        let raster = rasterize_text(&key.text, key.style.clone(), key.color);
        if cache.len() >= TEXT_RASTER_CACHE_LIMIT {
            cache.remove(0);
        }
        cache.push((key, raster.clone()));
        raster
    })
}

fn rasterize_text(text: &str, style: TextStyle, color: ClearColor) -> Option<TextRaster> {
    let mut spans = Vec::new();
    let mut bounds: Option<TextBounds> = None;

    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let super::context::FontContext {
            font_system,
            swash_cache,
        } = &mut *context;
        let mut cosmic_buffer = Buffer::new(font_system, font_metrics(&style));
        cosmic_buffer.set_wrap(font_system, Wrap::None);
        cosmic_buffer.set_size(font_system, None, None);
        let attrs = text_attrs(&style);
        cosmic_buffer.set_text(font_system, text, &attrs, cosmic_text::Shaping::Advanced);
        cosmic_buffer.shape_until_scroll(font_system, true);

        cosmic_buffer.draw(
            font_system,
            swash_cache,
            text_color(color),
            |pixel_x, pixel_y, width, height, pixel_color| {
                let pixel_color = modulate_alpha(pixel_color, color.alpha);
                if pixel_color.a() == 0 || width == 0 || height == 0 {
                    return;
                }

                let next = TextBounds {
                    left: pixel_x,
                    top: pixel_y,
                    right: pixel_x + width as i32,
                    bottom: pixel_y + height as i32,
                };
                bounds = Some(merge_bounds(bounds, next));
                spans.push(TextSpan {
                    x: pixel_x,
                    y: pixel_y,
                    width,
                    height,
                    color: pixel_color,
                });
            },
        );
    });

    let bounds = bounds?;
    let width = bounds.width().max(1) as u32;
    let height = bounds.height().max(1) as u32;
    let mut pixels = vec![0; width as usize * height as usize * 4];

    for span in spans {
        for offset_y in 0..span.height as i32 {
            for offset_x in 0..span.width as i32 {
                let x = span.x + offset_x - bounds.left;
                let y = span.y + offset_y - bounds.top;
                let offset = y as usize * width as usize * 4 + x as usize * 4;
                blend_pixel_bytes(&mut pixels[offset..offset + 4], span.color);
            }
        }
    }

    Some(TextRaster {
        bounds,
        width,
        pixels,
    })
}

fn draw_cached_text_raster(buffer: &mut impl PixelBuffer, x: i32, y: i32, raster: &TextRaster) {
    let size = buffer.size();
    let target_width = size.width as i32;
    let target_height = size.height as i32;
    let raster_height = raster.pixels.len() / raster.width as usize / 4;
    let target_pixels = buffer.pixels_mut();

    for source_y in 0..raster_height {
        let target_y = y + raster.bounds.top + source_y as i32;
        if target_y < 0 || target_y >= target_height {
            continue;
        }

        for source_x in 0..raster.width as usize {
            let target_x = x + raster.bounds.left + source_x as i32;
            if target_x < 0 || target_x >= target_width {
                continue;
            }

            let source_offset = (source_y * raster.width as usize + source_x) * 4;
            let target_offset = (target_y as usize * target_width as usize + target_x as usize) * 4;
            blend_argb8888_pixel(
                &mut target_pixels[target_offset..target_offset + 4],
                &raster.pixels[source_offset..source_offset + 4],
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TextSpan {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: cosmic_text::Color,
}

fn merge_bounds(current: Option<TextBounds>, next: TextBounds) -> TextBounds {
    match current {
        Some(current) => TextBounds {
            left: current.left.min(next.left),
            top: current.top.min(next.top),
            right: current.right.max(next.right),
            bottom: current.bottom.max(next.bottom),
        },
        None => next,
    }
}

fn blend_pixel_bytes(dst: &mut [u8], color: cosmic_text::Color) {
    let src_alpha = color.a() as u16;
    if src_alpha == 0 {
        return;
    }

    let src = [
        premultiply(color.b(), color.a()),
        premultiply(color.g(), color.a()),
        premultiply(color.r(), color.a()),
        color.a(),
    ];
    blend_argb8888_pixel(dst, &src);
}

fn blend_argb8888_pixel(dst: &mut [u8], src: &[u8]) {
    let src_alpha = u16::from(src[3]);
    if src_alpha == 0 {
        return;
    }

    if src_alpha == u16::from(u8::MAX) {
        dst.copy_from_slice(src);
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    dst[0] = blend_component(dst[0], src[0], inverse_alpha);
    dst[1] = blend_component(dst[1], src[1], inverse_alpha);
    dst[2] = blend_component(dst[2], src[2], inverse_alpha);
    dst[3] = blend_component(dst[3], src[3], inverse_alpha);
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}
