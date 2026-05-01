use std::sync::Arc;

use image::{Rgba, RgbaImage};

use super::{
    BackgroundAsset, BackgroundGradient, BackgroundKind, BackgroundLayered, BackgroundLayeredBase,
    BackgroundLayeredBlob, BackgroundRadial, BackgroundScaling, BackgroundTreatment,
    GeneratedBackground, RenderCacheSummary, SourceCacheStatus,
    asset::unique_sizes,
    render::{cover_dimensions, fit_dimensions},
};
use crate::{ClearColor, FrameSize};

#[test]
fn renders_solid_backgrounds() {
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(12, 16, 24),
        None,
        BackgroundTreatment::default(),
    )
    .expect("asset");
    let buffer = asset.render(FrameSize::new(2, 1)).expect("buffer");

    assert_eq!(buffer.pixels(), &[24, 16, 12, 255, 24, 16, 12, 255]);
}

#[test]
fn renders_bilinear_gradients() {
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(0, 0, 0),
        Some(GeneratedBackground::Gradient(BackgroundGradient {
            top_left: ClearColor::opaque(255, 0, 0),
            top_right: ClearColor::opaque(0, 255, 0),
            bottom_left: ClearColor::opaque(0, 0, 255),
            bottom_right: ClearColor::opaque(255, 255, 255),
        })),
        BackgroundTreatment::default(),
    )
    .expect("asset");
    let buffer = asset.render(FrameSize::new(2, 2)).expect("buffer");

    assert_eq!(&buffer.pixels()[0..4], &[0, 0, 255, 255]);
    assert_eq!(&buffer.pixels()[4..8], &[0, 255, 0, 255]);
    assert_eq!(&buffer.pixels()[8..12], &[255, 0, 0, 255]);
    assert_eq!(&buffer.pixels()[12..16], &[255, 255, 255, 255]);
}

#[test]
fn renders_radial_backgrounds() {
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(0, 0, 0),
        Some(GeneratedBackground::Radial(BackgroundRadial {
            center: ClearColor::opaque(255, 255, 255),
            edge: ClearColor::opaque(0, 0, 0),
            center_x: 50,
            center_y: 50,
            radius: 100,
        })),
        BackgroundTreatment::default(),
    )
    .expect("asset");
    let buffer = asset.render(FrameSize::new(3, 3)).expect("buffer");

    assert_eq!(&buffer.pixels()[16..20], &[255, 255, 255, 255]);
    assert_eq!(&buffer.pixels()[0..4], &[0, 0, 0, 255]);
}

#[test]
fn renders_layered_backgrounds() {
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(0, 0, 0),
        Some(GeneratedBackground::Layered(BackgroundLayered {
            base: BackgroundLayeredBase::Solid(ClearColor::opaque(0, 0, 0)),
            blobs: [
                Some(BackgroundLayeredBlob {
                    color: ClearColor::rgba(255, 0, 0, 128),
                    x: 50,
                    y: 50,
                    size: 40,
                }),
                None,
                None,
            ],
        })),
        BackgroundTreatment::default(),
    )
    .expect("asset");
    let buffer = asset.render(FrameSize::new(5, 5)).expect("buffer");

    assert_eq!(&buffer.pixels()[0..4], &[0, 0, 0, 255]);
    let center = &buffer.pixels()[48..52];
    assert!(center[2] > 0);
    assert_eq!(center[3], 255);
}

#[test]
fn scales_images_into_argb8888_buffers() {
    let mut image = RgbaImage::new(1, 1);
    image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
    let asset = BackgroundAsset {
        kind: BackgroundKind::Image {
            image: Arc::new(image),
            fallback: ClearColor::opaque(0, 0, 0),
        },
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
fn fit_dimensions_preserve_entire_image() {
    assert_eq!(fit_dimensions(4000, 3000, 1920, 1080), (1440, 1080));
    assert_eq!(fit_dimensions(3000, 4000, 1920, 1080), (810, 1080));
}

#[test]
fn fit_scaling_uses_fallback_bars() {
    let mut image = RgbaImage::new(2, 1);
    image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    image.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
    let asset = BackgroundAsset {
        kind: BackgroundKind::Image {
            image: Arc::new(image),
            fallback: ClearColor::opaque(10, 20, 30),
        },
        treatment: BackgroundTreatment {
            scaling: BackgroundScaling::Fit,
            ..BackgroundTreatment::default()
        },
    };

    let buffer = asset.render(FrameSize::new(4, 4)).expect("buffer");

    assert_eq!(&buffer.pixels()[0..4], &[30, 20, 10, 255]);
    let middle_left = 4 * 4usize;
    let middle_right = 7 * 4usize;
    assert_eq!(
        &buffer.pixels()[middle_left..middle_left + 4],
        &[0, 0, 255, 255]
    );
    assert_eq!(
        &buffer.pixels()[middle_right..middle_right + 4],
        &[0, 255, 0, 255]
    );
}

#[test]
fn center_scaling_keeps_image_unscaled() {
    let mut image = RgbaImage::new(1, 1);
    image.put_pixel(0, 0, Rgba([12, 34, 56, 255]));
    let asset = BackgroundAsset {
        kind: BackgroundKind::Image {
            image: Arc::new(image),
            fallback: ClearColor::opaque(1, 2, 3),
        },
        treatment: BackgroundTreatment {
            scaling: BackgroundScaling::Center,
            ..BackgroundTreatment::default()
        },
    };

    let buffer = asset.render(FrameSize::new(3, 3)).expect("buffer");

    assert_eq!(&buffer.pixels()[0..4], &[3, 2, 1, 255]);
    let center = 4 * 4usize;
    assert_eq!(&buffer.pixels()[center..center + 4], &[56, 34, 12, 255]);
}

#[test]
fn tile_scaling_repeats_source_image() {
    let mut image = RgbaImage::new(2, 1);
    image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    image.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
    let asset = BackgroundAsset {
        kind: BackgroundKind::Image {
            image: Arc::new(image),
            fallback: ClearColor::opaque(0, 0, 0),
        },
        treatment: BackgroundTreatment {
            scaling: BackgroundScaling::Tile,
            ..BackgroundTreatment::default()
        },
    };

    let buffer = asset.render(FrameSize::new(4, 2)).expect("buffer");

    assert_eq!(&buffer.pixels()[0..4], &[0, 0, 255, 255]);
    assert_eq!(&buffer.pixels()[4..8], &[0, 255, 0, 255]);
    assert_eq!(&buffer.pixels()[8..12], &[0, 0, 255, 255]);
    assert_eq!(&buffer.pixels()[12..16], &[0, 255, 0, 255]);
}

#[test]
fn stretch_scaling_fills_target_without_preserving_aspect() {
    let mut image = RgbaImage::new(1, 2);
    image.put_pixel(0, 0, Rgba([200, 10, 20, 255]));
    image.put_pixel(0, 1, Rgba([30, 220, 40, 255]));
    let asset = BackgroundAsset {
        kind: BackgroundKind::Image {
            image: Arc::new(image),
            fallback: ClearColor::opaque(0, 0, 0),
        },
        treatment: BackgroundTreatment {
            scaling: BackgroundScaling::Stretch,
            ..BackgroundTreatment::default()
        },
    };

    let buffer = asset.render(FrameSize::new(2, 2)).expect("buffer");

    assert_eq!(&buffer.pixels()[0..4], &[20, 10, 200, 255]);
    assert_eq!(&buffer.pixels()[4..8], &[20, 10, 200, 255]);
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
        None,
        BackgroundTreatment {
            blur_radius: 0,
            dim_strength: 20,
            tint: Some(ClearColor::opaque(10, 20, 40)),
            scaling: BackgroundScaling::Fill,
        },
    )
    .expect("asset");
    let buffer = asset.render(FrameSize::new(1, 1)).expect("buffer");

    assert_ne!(buffer.pixels(), &[140, 120, 100, 255]);
}
