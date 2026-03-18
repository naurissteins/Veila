use std::{path::Path, sync::Arc};

use image::{RgbaImage, imageops::FilterType};

use crate::{ClearColor, FrameSize, Result, SoftwareBuffer};

#[derive(Debug, Clone)]
pub struct BackgroundAsset {
    kind: BackgroundKind,
}

#[derive(Debug, Clone)]
enum BackgroundKind {
    Solid(ClearColor),
    Image(Arc<RgbaImage>),
}

impl BackgroundAsset {
    pub fn load(path: Option<&Path>, fallback: ClearColor) -> Result<Self> {
        match path {
            Some(path) => Ok(Self {
                kind: BackgroundKind::Image(Arc::new(image::open(path)?.to_rgba8())),
            }),
            None => Ok(Self {
                kind: BackgroundKind::Solid(fallback),
            }),
        }
    }

    pub fn render(&self, size: FrameSize) -> Result<SoftwareBuffer> {
        match &self.kind {
            BackgroundKind::Solid(color) => SoftwareBuffer::solid(size, *color),
            BackgroundKind::Image(image) => render_image(image, size),
        }
    }
}

fn render_image(image: &RgbaImage, size: FrameSize) -> Result<SoftwareBuffer> {
    let (scaled_width, scaled_height) = cover_dimensions(
        image.width(),
        image.height(),
        size.width.max(1),
        size.height.max(1),
    );
    let resized = image::imageops::resize(image, scaled_width, scaled_height, FilterType::Triangle);
    let crop_x = (scaled_width.saturating_sub(size.width)) / 2;
    let crop_y = (scaled_height.saturating_sub(size.height)) / 2;
    let cropped =
        image::imageops::crop_imm(&resized, crop_x, crop_y, size.width, size.height).to_image();
    let mut buffer = SoftwareBuffer::new(size)?;

    for (target, pixel) in buffer
        .pixels_mut()
        .chunks_exact_mut(4)
        .zip(cropped.pixels())
    {
        target.copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Ok(buffer)
}

fn cover_dimensions(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let width_limited_height =
        (u128::from(source_height) * u128::from(target_width)).div_ceil(u128::from(source_width));
    if width_limited_height >= u128::from(target_height) {
        return (target_width, width_limited_height as u32);
    }

    let height_limited_width =
        (u128::from(source_width) * u128::from(target_height)).div_ceil(u128::from(source_height));
    (height_limited_width as u32, target_height)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use image::{Rgba, RgbaImage};

    use super::{BackgroundAsset, BackgroundKind, cover_dimensions};
    use crate::{ClearColor, FrameSize};

    #[test]
    fn renders_solid_backgrounds() {
        let asset = BackgroundAsset::load(None, ClearColor::opaque(12, 16, 24)).expect("asset");
        let buffer = asset.render(FrameSize::new(2, 1)).expect("buffer");

        assert_eq!(buffer.pixels(), &[24, 16, 12, 255, 24, 16, 12, 255]);
    }

    #[test]
    fn scales_images_into_argb8888_buffers() {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
        let asset = BackgroundAsset {
            kind: BackgroundKind::Image(Arc::new(image)),
        };

        let buffer = asset.render(FrameSize::new(2, 1)).expect("buffer");

        assert_eq!(buffer.pixels(), &[30, 20, 10, 255, 30, 20, 10, 255]);
    }

    #[test]
    fn cover_dimensions_fill_target() {
        assert_eq!(cover_dimensions(4000, 3000, 1920, 1080), (1920, 1440));
        assert_eq!(cover_dimensions(3000, 4000, 1920, 1080), (1920, 2560));
    }
}
