use super::{
    BackdropLayerAlignment, BackdropLayerMode, BackdropLayerShape, BackdropLayerStyle,
    draw_backdrop_layer,
};
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
            BackdropLayerShape::Panel,
            ClearColor::rgba(255, 255, 255, 64),
            0,
            0,
            None,
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
        BackdropLayerStyle::new(
            BackdropLayerMode::Blur,
            BackdropLayerShape::Panel,
            ClearColor::rgba(8, 10, 14, 0),
            8,
            0,
            None,
            0,
        ),
    );

    assert_ne!(buffer.pixels(), before.as_slice());
}

#[test]
fn rounded_blur_layer_preserves_corner_pixels() {
    let mut buffer =
        SoftwareBuffer::solid(FrameSize::new(8, 8), ClearColor::opaque(0, 0, 0)).unwrap();
    fill_rect(
        &mut buffer,
        Rect::new(0, 0, 8, 8),
        ClearColor::opaque(255, 255, 255),
    );

    let before_corner = buffer.pixels()[..4].to_vec();
    draw_backdrop_layer(
        &mut buffer,
        Rect::new(0, 0, 8, 8),
        BackdropLayerStyle::new(
            BackdropLayerMode::Blur,
            BackdropLayerShape::Panel,
            ClearColor::rgba(8, 10, 14, 0),
            8,
            3,
            None,
            0,
        ),
    );

    assert_eq!(&buffer.pixels()[..4], before_corner.as_slice());
}

#[test]
fn draws_rounded_layer_border() {
    let mut buffer =
        SoftwareBuffer::solid(FrameSize::new(8, 8), ClearColor::opaque(0, 0, 0)).unwrap();

    draw_backdrop_layer(
        &mut buffer,
        Rect::new(1, 1, 6, 6),
        BackdropLayerStyle::new(
            BackdropLayerMode::Solid,
            BackdropLayerShape::Panel,
            ClearColor::rgba(8, 10, 14, 0),
            0,
            2,
            Some(ClearColor::opaque(255, 255, 255)),
            1,
        ),
    );

    assert!(buffer.pixels()[4 * (8 + 3) + 2] > 0);
}

#[test]
fn diagonal_layer_keeps_bottom_right_unfilled() {
    let mut buffer =
        SoftwareBuffer::solid(FrameSize::new(6, 6), ClearColor::opaque(0, 0, 0)).unwrap();

    draw_backdrop_layer(
        &mut buffer,
        Rect::new(0, 0, 6, 6),
        BackdropLayerStyle::new(
            BackdropLayerMode::Solid,
            BackdropLayerShape::Diagonal(BackdropLayerAlignment::Left),
            ClearColor::opaque(255, 0, 0),
            0,
            0,
            None,
            0,
        ),
    );

    assert_eq!(&buffer.pixels()[0..4], &[0, 0, 255, 255]);
    let bottom_right = ((5 * 6) + 5) * 4;
    assert_eq!(
        &buffer.pixels()[bottom_right..bottom_right + 4],
        &[0, 0, 0, 255]
    );
}
