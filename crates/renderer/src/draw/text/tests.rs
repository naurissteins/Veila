use super::{
    TextStyle, bundled_clock_font_family, bundled_clock_font_postscript_name, draw_text,
    draw_text_with_shadow, fit_wrapped_text, measure_text, resolve_font_family, wrap_text,
};
use crate::{ClearColor, FrameSize, ShadowStyle, SoftwareBuffer};

#[test]
fn measures_text_blocks() {
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
    let (width, height) = measure_text("AB", style);

    assert!(width > 0);
    assert!(height > 0);
}

#[test]
fn wraps_text_to_requested_width() {
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
    let block = wrap_text("one two three", style, 70);

    assert!(block.lines.len() > 1);
    assert!(block.width <= 70);
}

#[test]
fn reduces_scale_to_fit_narrow_widths() {
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 3);
    let block = fit_wrapped_text("W", style, 10, 1);

    assert!(block.style.scale < 3);
    assert!(block.width <= 10);
}

#[test]
fn renders_non_empty_text() {
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
    let mut buffer = SoftwareBuffer::new(FrameSize::new(64, 32)).expect("buffer");

    draw_text(&mut buffer, 0, 0, "K", style);

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn respects_text_alpha_when_rendering() {
    let mut faint =
        SoftwareBuffer::solid(FrameSize::new(64, 32), ClearColor::opaque(0, 0, 0)).expect("buffer");
    let mut opaque =
        SoftwareBuffer::solid(FrameSize::new(64, 32), ClearColor::opaque(0, 0, 0)).expect("buffer");

    draw_text(
        &mut faint,
        0,
        0,
        "88:88",
        TextStyle::new(ClearColor::rgba(255, 255, 255, 5), 3),
    );
    draw_text(
        &mut opaque,
        0,
        0,
        "88:88",
        TextStyle::new(ClearColor::opaque(255, 255, 255), 3),
    );

    let faint_total: u64 = faint
        .pixels()
        .chunks_exact(4)
        .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
        .sum();
    let opaque_total: u64 = opaque
        .pixels()
        .chunks_exact(4)
        .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
        .sum();

    assert!(faint_total > 0);
    assert!(faint_total < opaque_total);
}

#[test]
fn renders_shadowed_text() {
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
    let shadow = ShadowStyle::new(ClearColor::opaque(8, 10, 14), 2, 2);
    let mut buffer = SoftwareBuffer::new(FrameSize::new(64, 32)).expect("buffer");

    draw_text_with_shadow(&mut buffer, 0, 0, "K", style, shadow);

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn resolves_bundled_clock_font_family_from_loaded_database() {
    assert!(bundled_clock_font_family().is_some());
}

#[test]
fn resolves_postscript_font_name_to_family_name() {
    let family = bundled_clock_font_family().expect("bundled family");
    let postscript = bundled_clock_font_postscript_name().expect("bundled postscript name");

    assert_eq!(
        resolve_font_family(&postscript).as_deref(),
        Some(family.as_str())
    );
}

#[test]
fn resolves_bundled_weather_font_family_from_loaded_database() {
    assert_eq!(resolve_font_family("Geom").as_deref(), Some("Geom"));
}

#[test]
fn resolves_bundled_weather_postscript_name_to_family_name() {
    assert_eq!(
        resolve_font_family("Geom-SemiBold").as_deref(),
        Some("Geom")
    );
}
