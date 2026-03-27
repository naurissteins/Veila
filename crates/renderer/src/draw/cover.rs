use std::path::Path;

use image::RgbaImage;
use tiny_skia::{FillRule, FilterQuality, Mask, PathBuilder, Pixmap, PixmapPaint, Transform};

use crate::{FrameSize, RendererError, Result, SoftwareBuffer};

use super::skia::draw_overlay;

#[derive(Debug, Clone)]
pub enum CoverArtAsset {
    Image(Pixmap),
}

impl CoverArtAsset {
    pub fn load(path: &Path) -> Result<Self> {
        let image = image::open(path)?.to_rgba8();
        let pixmap = rgba_to_pixmap(image)?;
        Ok(Self::Image(pixmap))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        buffer: &mut SoftwareBuffer,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        radius: i32,
        opacity: Option<u8>,
    ) {
        if width == 0 || height == 0 {
            return;
        }

        let Self::Image(image) = self;
        draw_cover_image(buffer, left, top, width, height, radius, opacity, image);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_cover_image(
    buffer: &mut SoftwareBuffer,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    radius: i32,
    opacity: Option<u8>,
    image: &Pixmap,
) {
    draw_overlay(buffer, left, top, width, height, |overlay| {
        let Some(mut mask) = Mask::new(width, height) else {
            return;
        };
        let Some(path) = rounded_rect_path(width as i32, height as i32, radius) else {
            return;
        };
        mask.fill_path(&path, FillRule::Winding, true, Transform::identity());

        let paint = PixmapPaint {
            quality: FilterQuality::Bicubic,
            opacity: opacity.map_or(1.0, |opacity| f32::from(opacity.min(100)) / 100.0),
            ..PixmapPaint::default()
        };
        let scale = f32::max(
            width as f32 / image.width() as f32,
            height as f32 / image.height() as f32,
        );
        let translate_x = (width as f32 - image.width() as f32 * scale) / 2.0;
        let translate_y = (height as f32 - image.height() as f32 * scale) / 2.0;
        let transform = Transform::from_row(scale, 0.0, 0.0, scale, translate_x, translate_y);

        overlay.draw_pixmap(0, 0, image.as_ref(), &paint, transform, Some(&mask));
    });
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

fn rgba_to_pixmap(image: RgbaImage) -> Result<Pixmap> {
    let width = image.width();
    let height = image.height();
    let size = tiny_skia::IntSize::from_wh(width, height).ok_or(
        RendererError::InvalidFrameSize(FrameSize::new(width, height)),
    )?;
    let mut data = image.into_raw();
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3];
        pixel[0] = premultiply(pixel[0], alpha);
        pixel[1] = premultiply(pixel[1], alpha);
        pixel[2] = premultiply(pixel[2], alpha);
    }
    Pixmap::from_vec(data, size).ok_or(RendererError::InvalidFrameSize(FrameSize::new(
        width, height,
    )))
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{CoverArtAsset, rgba_to_pixmap};
    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    #[test]
    fn converts_rgba_cover_to_pixmap() {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([90, 120, 180, 255]));
        let pixmap = rgba_to_pixmap(image).expect("pixmap");

        assert_eq!(pixmap.data(), &[90, 120, 180, 255]);
    }

    #[test]
    fn draws_cover_art_into_buffer() {
        let mut image = RgbaImage::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                image.put_pixel(x, y, Rgba([255, 180, 90, 255]));
            }
        }
        let pixmap = rgba_to_pixmap(image).expect("pixmap");
        let asset = CoverArtAsset::Image(pixmap);
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(80, 80), ClearColor::opaque(0, 0, 0)).unwrap();

        asset.draw(&mut buffer, 8, 8, 48, 48, 12, None);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
