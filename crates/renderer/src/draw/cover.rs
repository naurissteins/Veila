use std::{io::BufReader, path::Path, sync::Arc};

use image::{ImageReader, Limits, RgbaImage, imageops::FilterType};
use tiny_skia::{FillRule, FilterQuality, Mask, PathBuilder, Pixmap, PixmapPaint, Transform};

use crate::{FrameSize, PixelBuffer, RendererError, Result};

use super::skia::draw_overlay;

#[derive(Debug, Clone)]
pub enum CoverArtAsset {
    Image(Arc<Pixmap>),
}

const MAX_COVER_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COVER_SOURCE_DIMENSION: u32 = 4_096;
const MAX_COVER_PIXMAP_DIMENSION: u32 = 2_048;

impl CoverArtAsset {
    pub fn load(path: &Path, max_dimension: u32) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        if file.metadata()?.len() > MAX_COVER_FILE_BYTES {
            return Err(RendererError::Io(std::io::Error::other(
                "cover art file exceeds the size limit",
            )));
        }
        let mut reader = ImageReader::new(BufReader::new(file)).with_guessed_format()?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_COVER_SOURCE_DIMENSION);
        limits.max_image_height = Some(MAX_COVER_SOURCE_DIMENSION);
        limits.max_alloc = Some(MAX_COVER_FILE_BYTES);
        reader.limits(limits);
        let image = reader.decode()?;
        let side = image.width().min(image.height());
        let left = (image.width() - side) / 2;
        let top = (image.height() - side) / 2;
        let square = image.crop_imm(left, top, side, side);
        let max_dimension = max_dimension.clamp(1, MAX_COVER_PIXMAP_DIMENSION);
        let prepared = if side > max_dimension {
            square.resize_exact(max_dimension, max_dimension, FilterType::Triangle)
        } else {
            square
        };
        let pixmap = rgba_to_pixmap(prepared.to_rgba8())?;
        Ok(Self::Image(Arc::new(pixmap)))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        buffer: &mut impl PixelBuffer,
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
    buffer: &mut impl PixelBuffer,
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
    for pixel in data.as_chunks_mut::<4>().0 {
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
    use std::fs;

    use image::{Rgba, RgbaImage};

    use super::{CoverArtAsset, rgba_to_pixmap};
    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    const ONE_PIXEL_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

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
        let asset = CoverArtAsset::Image(std::sync::Arc::new(pixmap));
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(80, 80), ClearColor::opaque(0, 0, 0)).unwrap();

        asset.draw(&mut buffer, 8, 8, 48, 48, 12, None);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn loads_png_cover_art_without_file_extension() {
        let path = std::env::temp_dir().join(format!(
            "veila-cover-art-extensionless-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(&path, ONE_PIXEL_PNG).expect("write png");

        let result = CoverArtAsset::load(&path, 64);

        let _ = fs::remove_file(path);
        assert!(result.is_ok());
    }

    #[test]
    fn bounds_prepared_artwork_to_widget_size() {
        let path =
            std::env::temp_dir().join(format!("veila-cover-size-{}.png", std::process::id()));
        let image = RgbaImage::from_pixel(128, 64, Rgba([255, 0, 0, 255]));
        image.save(&path).expect("save artwork");

        let asset = CoverArtAsset::load(&path, 32).expect("load artwork");
        let _ = fs::remove_file(path);

        let CoverArtAsset::Image(pixmap) = asset;
        assert_eq!((pixmap.width(), pixmap.height()), (32, 32));
    }

    #[test]
    fn rejects_artwork_above_source_dimension_limit() {
        let path =
            std::env::temp_dir().join(format!("veila-cover-limit-{}.png", std::process::id()));
        let image = RgbaImage::from_pixel(4_097, 1, Rgba([255, 0, 0, 255]));
        image.save(&path).expect("save artwork");

        let result = CoverArtAsset::load(&path, 32);
        let _ = fs::remove_file(path);

        assert!(result.is_err());
    }
}
