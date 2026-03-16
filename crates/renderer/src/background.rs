use std::path::Path;

use image::{RgbaImage, imageops::FilterType};

use crate::{ClearColor, FrameSize, Result, SoftwareBuffer};

#[derive(Debug, Clone)]
pub struct BackgroundAsset {
    kind: BackgroundKind,
}

#[derive(Debug, Clone)]
enum BackgroundKind {
    Solid(ClearColor),
    Image(RgbaImage),
}

impl BackgroundAsset {
    pub fn load(path: Option<&Path>, fallback: ClearColor) -> Result<Self> {
        match path {
            Some(path) => Ok(Self {
                kind: BackgroundKind::Image(image::open(path)?.to_rgba8()),
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
    let resized = image::DynamicImage::ImageRgba8(image.clone())
        .resize_to_fill(size.width.max(1), size.height.max(1), FilterType::Triangle)
        .to_rgba8();
    let mut buffer = SoftwareBuffer::new(size)?;

    for (target, pixel) in buffer
        .pixels_mut()
        .chunks_exact_mut(4)
        .zip(resized.pixels())
    {
        target.copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{BackgroundAsset, BackgroundKind};
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
            kind: BackgroundKind::Image(image),
        };

        let buffer = asset.render(FrameSize::new(2, 1)).expect("buffer");

        assert_eq!(buffer.pixels(), &[30, 20, 10, 255, 30, 20, 10, 255]);
    }
}
