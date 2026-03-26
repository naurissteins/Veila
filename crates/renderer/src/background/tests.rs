use std::sync::Arc;

use image::{Rgba, RgbaImage};

use super::{
    BackgroundAsset, BackgroundKind, BackgroundTreatment, RenderCacheSummary, SourceCacheStatus,
    asset::unique_sizes, render::cover_dimensions,
};
use crate::{ClearColor, FrameSize};

#[test]
fn renders_solid_backgrounds() {
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(12, 16, 24),
        BackgroundTreatment::default(),
    )
    .expect("asset");
    let buffer = asset.render(FrameSize::new(2, 1)).expect("buffer");

    assert_eq!(buffer.pixels(), &[24, 16, 12, 255, 24, 16, 12, 255]);
}

#[test]
fn scales_images_into_argb8888_buffers() {
    let mut image = RgbaImage::new(1, 1);
    image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
    let asset = BackgroundAsset {
        kind: BackgroundKind::Image(Arc::new(image)),
        treatment: BackgroundTreatment::default(),
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

#[test]
fn applies_dim_and_tint_treatment() {
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(100, 120, 140),
        BackgroundTreatment {
            blur_radius: 0,
            dim_strength: 20,
            tint: Some(ClearColor::opaque(10, 20, 40)),
            tint_opacity: 10,
        },
    )
    .expect("asset");
    let buffer = asset.render(FrameSize::new(1, 1)).expect("buffer");

    assert_ne!(buffer.pixels(), &[140, 120, 100, 255]);
}
