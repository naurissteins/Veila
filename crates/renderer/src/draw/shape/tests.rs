use super::{
    BorderStyle, BoxStyle, CircleStyle, PillStyle, Rect, draw_box, draw_circle, draw_pill,
    fill_rect, stroke_rect,
};
use crate::{ClearColor, FrameSize, SoftwareBuffer};

#[test]
fn fills_rectangles() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(8, 8)).expect("buffer");
    fill_rect(
        &mut buffer,
        Rect::new(2, 2, 3, 3),
        ClearColor::opaque(255, 255, 255),
    );

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn blends_translucent_rectangles_over_existing_pixels() {
    let mut buffer = SoftwareBuffer::solid(FrameSize::new(1, 1), ClearColor::opaque(10, 20, 30))
        .expect("buffer");
    fill_rect(
        &mut buffer,
        Rect::new(0, 0, 1, 1),
        ClearColor::rgba(255, 255, 255, 128),
    );

    assert_eq!(buffer.pixels(), &[143, 138, 133, 255]);
}

#[test]
fn strokes_rectangles() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(8, 8)).expect("buffer");
    stroke_rect(
        &mut buffer,
        Rect::new(1, 1, 6, 6),
        BorderStyle::new(ClearColor::opaque(255, 255, 255), 1),
    );

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn draws_box_with_border() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(12, 12)).expect("buffer");
    draw_box(
        &mut buffer,
        Rect::new(1, 1, 10, 10),
        BoxStyle::new(ClearColor::opaque(8, 10, 14))
            .with_border(BorderStyle::new(ClearColor::opaque(255, 255, 255), 1)),
    );

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn draws_pill_surface() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 80)).expect("buffer");
    draw_pill(
        &mut buffer,
        Rect::new(16, 24, 88, 32),
        PillStyle::new(ClearColor::rgba(12, 18, 28, 210))
            .with_border(BorderStyle::new(ClearColor::opaque(255, 255, 255), 2)),
    );

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn draws_pill_surface_at_large_offsets() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(960, 540)).expect("buffer");
    draw_pill(
        &mut buffer,
        Rect::new(320, 240, 280, 56),
        PillStyle::new(ClearColor::rgba(12, 18, 28, 232))
            .with_border(BorderStyle::new(ClearColor::opaque(92, 108, 146), 2)),
    );

    let row_start = (268 * 960 + 460) * 4;
    assert_ne!(&buffer.pixels()[row_start..row_start + 4], &[0, 0, 0, 0]);
}

#[test]
fn draws_pill_surface_with_custom_radius() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(96, 72)).expect("buffer");
    draw_pill(
        &mut buffer,
        Rect::new(12, 16, 72, 32),
        PillStyle::new(ClearColor::rgba(12, 18, 28, 232))
            .with_border(BorderStyle::new(ClearColor::opaque(92, 108, 146), 2))
            .with_radius(10),
    );

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn blends_translucent_pill_fill_without_overdarkening() {
    let mut buffer = SoftwareBuffer::solid(FrameSize::new(24, 24), ClearColor::opaque(200, 100, 0))
        .expect("buffer");
    draw_pill(
        &mut buffer,
        Rect::new(4, 4, 16, 16),
        PillStyle::new(ClearColor::rgba(255, 255, 255, 51)),
    );

    let center = (12 * 24 + 12) * 4;
    assert_eq!(&buffer.pixels()[center..center + 4], &[51, 131, 211, 255]);
}

#[test]
fn draws_circle_surface() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(80, 80)).expect("buffer");
    draw_circle(
        &mut buffer,
        40,
        40,
        20,
        CircleStyle::new(ClearColor::rgba(240, 244, 250, 220))
            .with_border(BorderStyle::new(ClearColor::opaque(20, 24, 32), 2)),
    );

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn keeps_translucent_circle_fill_free_from_border_tint() {
    let mut buffer = SoftwareBuffer::solid(FrameSize::new(64, 64), ClearColor::opaque(200, 100, 0))
        .expect("buffer");
    draw_circle(
        &mut buffer,
        32,
        32,
        20,
        CircleStyle::new(ClearColor::rgba(255, 255, 255, 15))
            .with_border(BorderStyle::new(ClearColor::rgba(148, 178, 255, 108), 2)),
    );

    let center = (32 * 64 + 32) * 4;
    assert_eq!(&buffer.pixels()[center..center + 4], &[15, 109, 203, 255]);
}
