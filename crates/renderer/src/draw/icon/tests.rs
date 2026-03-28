use super::{
    AssetIcon, BatteryIcon, ICON_RASTER_CACHE, IconRasterKey, IconStyle, WeatherIcon, draw_icon,
    icon_source, icon_visible_bounds,
    parser::extract_path_data,
    parser::extract_viewbox,
    parser::parse_path_data,
    raster::{rasterize_icon, scale_svg_alpha, svg_translate_y, visible_alpha_bounds},
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

#[test]
fn weather_svg_icons_preserve_source_fill_colors() {
    let key = IconRasterKey {
        icon: AssetIcon::Weather(WeatherIcon::ClearDay),
        width: 48,
        height: 48,
        color: ClearColor::opaque(255, 255, 255),
        padding: 0,
    };
    let pixels = rasterize_icon(key, icon_source(key.icon));

    assert!(
        pixels
            .chunks_exact(4)
            .any(|pixel| { pixel[3] > 0 && (pixel[0] < 240 || pixel[1] < 240 || pixel[2] < 240) })
    );
}

#[test]
fn battery_svg_icons_follow_style_color() {
    let key = IconRasterKey {
        icon: AssetIcon::Battery(BatteryIcon::Full),
        width: 48,
        height: 48,
        color: ClearColor::opaque(255, 255, 255),
        padding: 0,
    };
    let pixels = rasterize_icon(key, icon_source(key.icon));

    assert!(
        pixels
            .chunks_exact(4)
            .any(|pixel| { pixel[3] > 0 && pixel[0] > 220 && pixel[1] > 220 && pixel[2] > 220 })
    );
}

#[test]
fn weather_svg_icons_trim_internal_transparent_bounds() {
    let key = IconRasterKey {
        icon: AssetIcon::Weather(WeatherIcon::ClearDay),
        width: 64,
        height: 64,
        color: ClearColor::opaque(255, 255, 255),
        padding: 0,
    };

    let pixels = rasterize_icon(key, icon_source(key.icon));
    let bounds = visible_alpha_bounds(&pixels, key.width, key.height).expect("alpha bounds");

    assert!(bounds.width() >= 60);
    assert!(bounds.height() >= 60);
    assert!(bounds.left <= 2);
    assert!(bounds.top <= 2);
    assert!(key.width - bounds.right <= 2);
    assert!(key.height - bounds.bottom <= 2);
}

#[test]
fn reports_visible_weather_icon_bounds_inside_requested_rect() {
    let rect = Rect::new(10, 20, 100, 100);
    let bounds = icon_visible_bounds(
        rect,
        AssetIcon::Weather(WeatherIcon::Cloudy),
        IconStyle::new(ClearColor::opaque(255, 255, 255)).with_padding(0),
    )
    .expect("visible bounds");

    assert!(bounds.y >= rect.y);
    assert!(bounds.y + bounds.height <= rect.y + rect.height);
    assert!(bounds.height < rect.height);
}

#[test]
fn wide_weather_icons_use_bottom_aligned_svg_translation() {
    let weather_key = IconRasterKey {
        icon: AssetIcon::Weather(WeatherIcon::Cloudy),
        width: 64,
        height: 64,
        color: ClearColor::opaque(255, 255, 255),
        padding: 0,
    };
    let generic_key = IconRasterKey {
        icon: AssetIcon::Eye,
        ..weather_key
    };

    assert!(svg_translate_y(weather_key, 40.0) > svg_translate_y(generic_key, 40.0));
    assert_eq!(svg_translate_y(weather_key, 40.0), 24.0);
    assert_eq!(svg_translate_y(generic_key, 40.0), 12.0);
}

#[test]
fn weather_svg_icons_scale_alpha_without_recoloring() {
    let mut pixels = vec![0, 0, 0, 255, 20, 40, 60, 128];
    scale_svg_alpha(&mut pixels, 128);

    assert_eq!(pixels[0], 0);
    assert_eq!(pixels[1], 0);
    assert_eq!(pixels[2], 0);
    assert_eq!(pixels[3], 128);
    assert_eq!(pixels[4], 10);
    assert_eq!(pixels[5], 20);
    assert_eq!(pixels[6], 30);
    assert_eq!(pixels[7], 64);
}
