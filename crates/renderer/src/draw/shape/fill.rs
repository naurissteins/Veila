use crate::{ClearColor, SoftwareBuffer};

use super::{BorderStyle, BoxStyle, Rect};

/// Fills a rectangle.
pub fn fill_rect(buffer: &mut SoftwareBuffer, rect: Rect, color: ClearColor) {
    if rect.is_empty() {
        return;
    }

    let size = buffer.size();
    let right = (rect.x + rect.width).clamp(0, size.width as i32);
    let bottom = (rect.y + rect.height).clamp(0, size.height as i32);
    let left = rect.x.clamp(0, size.width as i32);
    let top = rect.y.clamp(0, size.height as i32);

    if left >= right || top >= bottom {
        return;
    }

    let stride = size.width as usize * 4;
    let pixel = color.to_argb8888_bytes();
    let alpha = pixel[3];
    let pixels = buffer.pixels_mut();

    for row in top as usize..bottom as usize {
        let row_start = row * stride;
        for column in left as usize..right as usize {
            let offset = row_start + column * 4;
            if alpha == u8::MAX {
                pixels[offset..offset + 4].copy_from_slice(&pixel);
            } else {
                blend_argb8888_pixel(&mut pixels[offset..offset + 4], &pixel);
            }
        }
    }
}

/// Draws a border around a rectangle.
pub fn stroke_rect(buffer: &mut SoftwareBuffer, rect: Rect, border: BorderStyle) {
    if rect.is_empty() || border.thickness <= 0 {
        return;
    }

    fill_rect(
        buffer,
        Rect::new(rect.x, rect.y, rect.width, border.thickness),
        border.color,
    );
    fill_rect(
        buffer,
        Rect::new(
            rect.x,
            rect.y + rect.height - border.thickness,
            rect.width,
            border.thickness,
        ),
        border.color,
    );
    fill_rect(
        buffer,
        Rect::new(rect.x, rect.y, border.thickness, rect.height),
        border.color,
    );
    fill_rect(
        buffer,
        Rect::new(
            rect.x + rect.width - border.thickness,
            rect.y,
            border.thickness,
            rect.height,
        ),
        border.color,
    );
}

/// Draws a filled box with an optional border.
pub fn draw_box(buffer: &mut SoftwareBuffer, rect: Rect, style: BoxStyle) {
    fill_rect(buffer, rect, style.fill);

    if let Some(border) = style.border {
        stroke_rect(buffer, rect, border);
    }
}

fn blend_argb8888_pixel(dst: &mut [u8], src: &[u8; 4]) {
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

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}
