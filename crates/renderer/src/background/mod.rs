mod render_cache;
mod source_cache;

use std::{path::Path, sync::Arc};

use image::{RgbaImage, imageops::FilterType};
use render_cache::{load_cached_buffer, store_cached_buffer};
use source_cache::{load_cached_rgba, store_cached_rgba};

use crate::{ClearColor, FrameSize, Result, SoftwareBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCacheStatus {
    Hit,
    Warmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCacheSummary {
    pub cache_hits: usize,
    pub warmed_sizes: usize,
}

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
                kind: BackgroundKind::Image(Arc::new(load_rgba_image(path)?)),
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

pub fn prewarm_source(path: &Path) -> Result<SourceCacheStatus> {
    if load_cached_rgba(path)?.is_some() {
        return Ok(SourceCacheStatus::Hit);
    }

    let image = image::open(path)?.to_rgba8();
    store_cached_rgba(path, &image)?;
    Ok(SourceCacheStatus::Warmed)
}

pub fn load_cached_render(path: &Path, size: FrameSize) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer(path, size)
}

pub fn store_cached_render(path: &Path, size: FrameSize, buffer: &SoftwareBuffer) -> Result<()> {
    store_cached_buffer(path, size, buffer)
}

pub fn prewarm_rendered(
    path: &Path,
    fallback: ClearColor,
    sizes: &[FrameSize],
) -> Result<RenderCacheSummary> {
    let unique_sizes = unique_sizes(sizes);
    let mut cache_hits = 0;
    let mut missing_sizes = Vec::new();

    for size in unique_sizes {
        if load_cached_render(path, size)?.is_some() {
            cache_hits += 1;
        } else {
            missing_sizes.push(size);
        }
    }

    if missing_sizes.is_empty() {
        return Ok(RenderCacheSummary {
            cache_hits,
            warmed_sizes: 0,
        });
    }

    let asset = BackgroundAsset::load(Some(path), fallback)?;
    for size in &missing_sizes {
        let buffer = asset.render(*size)?;
        store_cached_render(path, *size, &buffer)?;
    }

    Ok(RenderCacheSummary {
        cache_hits,
        warmed_sizes: missing_sizes.len(),
    })
}

fn load_rgba_image(path: &Path) -> Result<RgbaImage> {
    if let Some(image) = load_cached_rgba(path)? {
        return Ok(image);
    }

    let image = image::open(path)?.to_rgba8();
    let _ = store_cached_rgba(path, &image);
    Ok(image)
}

fn unique_sizes(sizes: &[FrameSize]) -> Vec<FrameSize> {
    let mut unique = Vec::with_capacity(sizes.len());

    for size in sizes {
        if !unique.contains(size) {
            unique.push(*size);
        }
    }

    unique
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

    use super::{
        BackgroundAsset, BackgroundKind, RenderCacheSummary, SourceCacheStatus, cover_dimensions,
        unique_sizes,
    };
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

    #[test]
    fn source_cache_status_is_comparable() {
        assert_eq!(SourceCacheStatus::Hit, SourceCacheStatus::Hit);
    }

    #[test]
    fn deduplicates_render_sizes() {
        assert_eq!(
            unique_sizes(&[
                FrameSize::new(1920, 1080),
                FrameSize::new(1920, 1080),
                FrameSize::new(2560, 1440),
            ]),
            vec![FrameSize::new(1920, 1080), FrameSize::new(2560, 1440)]
        );
    }

    #[test]
    fn render_cache_summary_is_comparable() {
        assert_eq!(
            RenderCacheSummary {
                cache_hits: 1,
                warmed_sizes: 2,
            },
            RenderCacheSummary {
                cache_hits: 1,
                warmed_sizes: 2,
            }
        );
    }
}
