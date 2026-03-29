use image::{RgbaImage, imageops};

use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

use super::shape::fill_rect;

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
}

impl BackdropLayerStyle {
    pub const fn new(mode: BackdropLayerMode, color: ClearColor, blur_radius: u8) -> Self {
        Self {
            mode,
            color,
            blur_radius,
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
            fill_rect(buffer, clipped, style.color);
        }
        BackdropLayerMode::Blur => {
            blur_region(buffer, clipped, style.blur_radius);
            if style.color.alpha > 0 {
                fill_rect(buffer, clipped, style.color);
            }
        }
    }
}

fn blur_region(buffer: &mut SoftwareBuffer, rect: Rect, blur_radius: u8) {
    let width = rect.width.max(0) as u32;
    let height = rect.height.max(0) as u32;
    if width == 0 || height == 0 {
        return;
    }

    let rgba = extract_rgba_region(buffer, rect);
    let Some(region) = RgbaImage::from_raw(width, height, rgba) else {
        return;
    };
    let blurred = if blur_radius == 0 {
        region
    } else {
        imageops::blur(&region, f32::from(blur_radius.min(24)))
    };
    write_rgba_region(buffer, rect, &blurred);
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
            BackdropLayerStyle::new(BackdropLayerMode::Blur, ClearColor::rgba(8, 10, 14, 0), 8),
        );

        assert_ne!(buffer.pixels(), before.as_slice());
    }
}
