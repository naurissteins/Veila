use crate::blend_component;

use std::{cell::RefCell, rc::Rc, thread_local};

use cosmic_text::{Buffer, Wrap};
use zeroize::{Zeroize, Zeroizing};

use crate::PixelBuffer;

use super::{
    ClearColor, TextBounds, TextStyle, context::FONT_CONTEXT, font_metrics, modulate_alpha,
    text_attrs, text_color,
};

const TEXT_RASTER_CACHE_LIMIT: usize = 128;

thread_local! {
    static TEXT_RASTER_CACHE: RefCell<TextRasterCache> = const { RefCell::new(TextRasterCache { entries: Vec::new() }) };
}

pub(super) fn clear_text_raster_cache() {
    TEXT_RASTER_CACHE.with(|cache| cache.borrow_mut().entries.clear());
}

struct TextRasterCache {
    entries: Vec<(TextRasterKey, Option<Rc<TextRaster>>)>,
}

impl TextRasterCache {
    fn get(
        &mut self,
        text: &str,
        style: &TextStyle,
        color: ClearColor,
    ) -> Option<Option<Rc<TextRaster>>> {
        let index = self
            .entries
            .iter()
            .position(|(key, _)| key.text == text && key.style == *style && key.color == color)?;
        let entry = self.entries.remove(index);
        let raster = entry.1.clone();
        self.entries.push(entry);
        Some(raster)
    }

    fn bounds(&mut self, text: &str, style: &TextStyle) -> Option<TextBounds> {
        let index = self.entries.iter().position(|(key, raster)| {
            key.text == text && key.style == *style && raster.is_some()
        })?;
        let entry = self.entries.remove(index);
        let bounds = entry.1.as_ref().map(|raster| raster.bounds);
        self.entries.push(entry);
        bounds
    }

    fn insert(&mut self, key: TextRasterKey, raster: Option<Rc<TextRaster>>) {
        if self.entries.len() >= TEXT_RASTER_CACHE_LIMIT {
            self.entries.remove(0);
        }
        self.entries.push((key, raster));
    }
}

#[derive(Debug)]
struct TextRasterKey {
    text: String,
    style: TextStyle,
    color: ClearColor,
}

#[derive(Debug)]
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

pub(super) fn draw_sensitive_text_lines(
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

    let text = Zeroizing::new(lines.join("\n"));
    let Some(mut raster) = rasterize_text(&text, style, color) else {
        return;
    };

    draw_cached_text_raster(buffer, x, y, &raster);
    raster.pixels.zeroize();
}

pub(super) fn visible_text_bounds(text: &str, style: TextStyle) -> Option<TextBounds> {
    if text.is_empty() {
        return None;
    }

    if let Some(cached) = TEXT_RASTER_CACHE.with(|cache| cache.borrow_mut().bounds(text, &style)) {
        return Some(cached);
    }

    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let super::context::FontContext {
            font_system,
            swash_cache,
            ..
        } = &mut *context;
        let mut cosmic_buffer = Buffer::new(font_system, font_metrics(&style));
        cosmic_buffer.set_wrap(Wrap::None);
        cosmic_buffer.set_size(None, None);
        let attrs = text_attrs(&style);
        cosmic_buffer.set_text(text, &attrs, cosmic_text::Shaping::Advanced, None);
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

fn cached_text_raster(text: &str, style: TextStyle, color: ClearColor) -> Option<Rc<TextRaster>> {
    if let Some(raster) =
        TEXT_RASTER_CACHE.with(|cache| cache.borrow_mut().get(text, &style, color))
    {
        return raster;
    }

    let raster = rasterize_text(text, style.clone(), color).map(Rc::new);
    let key = TextRasterKey {
        text: text.to_owned(),
        style,
        color,
    };
    TEXT_RASTER_CACHE.with(|cache| cache.borrow_mut().insert(key, raster.clone()));
    raster
}

fn rasterize_text(text: &str, style: TextStyle, color: ClearColor) -> Option<TextRaster> {
    let mut spans: Vec<TextSpan> = Vec::new();

    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let super::context::FontContext {
            font_system,
            swash_cache,
            ..
        } = &mut *context;
        let mut cosmic_buffer = Buffer::new(font_system, font_metrics(&style));
        cosmic_buffer.set_wrap(Wrap::None);
        cosmic_buffer.set_size(None, None);
        let attrs = text_attrs(&style);
        cosmic_buffer.set_text(text, &attrs, cosmic_text::Shaping::Advanced, None);
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

                // Swash emits glyphs pixel by pixel, so collapse equal coverage along each row.
                if let Some(last) = spans.last_mut()
                    && last.y == pixel_y
                    && last.height == height
                    && last.color == pixel_color
                    && last.x + last.width as i32 == pixel_x
                {
                    last.width += width;
                    return;
                }
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

    let bounds = spans.iter().fold(None, |current, span| {
        Some(merge_bounds(current, span.bounds()))
    })?;
    let width = bounds.width().max(1) as u32;
    let height = bounds.height().max(1) as u32;
    let mut pixels = vec![0; width as usize * height as usize * 4];

    let stride = width as usize * 4;
    for span in spans {
        let color = span.premultiplied_color();
        let x = (span.x - bounds.left) as usize * 4;
        let top = (span.y - bounds.top) as usize;
        let span_bytes = span.width as usize * 4;
        for y in top..top + span.height as usize {
            let start = y * stride + x;
            for pixel in pixels[start..start + span_bytes].as_chunks_mut::<4>().0 {
                blend_argb8888_pixel(pixel, &color);
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

impl TextSpan {
    fn bounds(self) -> TextBounds {
        TextBounds {
            left: self.x,
            top: self.y,
            right: self.x + self.width as i32,
            bottom: self.y + self.height as i32,
        }
    }

    fn premultiplied_color(self) -> [u8; 4] {
        let alpha = self.color.a();
        [
            premultiply(self.color.b(), alpha),
            premultiply(self.color.g(), alpha),
            premultiply(self.color.r(), alpha),
            alpha,
        ]
    }
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

#[cfg(test)]
mod tests;
