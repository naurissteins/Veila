use crate::{ClearColor, FrameSize, SoftwareBuffer};

use super::{clear_color_from_bgra_pixel, solid_clear_pixel_from_buffer};

#[test]
fn detects_solid_clear_color() {
    let buffer =
        SoftwareBuffer::solid(FrameSize::new(2, 1), ClearColor::opaque(10, 20, 30)).unwrap();

    let pixel = solid_clear_pixel_from_buffer(&buffer).expect("solid color");

    assert_eq!(pixel, [30, 20, 10, 255]);
}

#[test]
fn rejects_non_solid_clear_color() {
    let buffer = SoftwareBuffer::from_argb8888_pixels(
        FrameSize::new(2, 1),
        vec![0, 0, 0, 255, 10, 20, 30, 255],
    )
    .unwrap();

    assert!(solid_clear_pixel_from_buffer(&buffer).is_none());
}

#[test]
fn converts_srgb_clear_color_to_linear() {
    let color =
        clear_color_from_bgra_pixel([128, 128, 128, 255], wgpu::TextureFormat::Bgra8UnormSrgb);

    assert!((color.r - 0.215_860_5).abs() < 0.000_001);
    assert!((color.g - 0.215_860_5).abs() < 0.000_001);
    assert!((color.b - 0.215_860_5).abs() < 0.000_001);
    assert_eq!(color.a, 1.0);
}
