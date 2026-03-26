use super::{
    AssetIcon, ICON_RASTER_CACHE, IconStyle, draw_icon, parser::extract_path_data,
    parser::extract_viewbox, parser::parse_path_data,
};
use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

#[test]
fn extracts_svg_path_data() {
    let data = extract_path_data(include_str!(
        "../../../../../assets/icons/eye-solid-full.svg"
    ));

    assert!(data.is_some());
}

#[test]
fn parses_svg_path_data() {
    let data = extract_path_data(include_str!(
        "../../../../../assets/icons/eye-solid-full.svg"
    ))
    .expect("path data");

    assert!(parse_path_data(data).is_some());
}

#[test]
fn extracts_svg_viewbox() {
    let viewbox = extract_viewbox(include_str!(
        "../../../../../assets/icons/eye-solid-full.svg"
    ))
    .expect("viewbox");

    assert_eq!(viewbox.width, 640.0);
    assert_eq!(viewbox.height, 640.0);
}

#[test]
fn renders_vector_eye_icon() {
    let mut buffer = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");
    draw_icon(
        &mut buffer,
        Rect::new(0, 0, 32, 32),
        AssetIcon::Eye,
        IconStyle::new(ClearColor::opaque(255, 255, 255)),
    );

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn vector_icons_are_distinct() {
    let mut eye = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");
    let mut eye_off = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");

    draw_icon(
        &mut eye,
        Rect::new(0, 0, 32, 32),
        AssetIcon::Eye,
        IconStyle::new(ClearColor::opaque(255, 255, 255)),
    );
    draw_icon(
        &mut eye_off,
        Rect::new(0, 0, 32, 32),
        AssetIcon::EyeOff,
        IconStyle::new(ClearColor::opaque(255, 255, 255)),
    );

    assert_ne!(eye.pixels(), eye_off.pixels());
}

#[test]
fn reuses_cached_raster_for_matching_icon_draws() {
    ICON_RASTER_CACHE.with(|cache| cache.borrow_mut().clear());
    let mut buffer = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");
    let style = IconStyle::new(ClearColor::opaque(255, 255, 255)).with_padding(4);

    draw_icon(&mut buffer, Rect::new(0, 0, 24, 24), AssetIcon::Eye, style);
    draw_icon(&mut buffer, Rect::new(0, 0, 24, 24), AssetIcon::Eye, style);

    ICON_RASTER_CACHE.with(|cache| {
        assert_eq!(cache.borrow().len(), 1);
    });
}
