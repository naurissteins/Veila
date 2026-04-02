use super::*;

#[test]
fn text_layout_cache_uses_configured_weather_icon_size() {
    let mut cache = TextLayoutCache::default();
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);

    let blocks = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_offset_x: None,
        clock_meridiem_offset_y: None,
        date_text: Some("Tuesday"),
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: None,
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: Some("Type your password to unlock"),
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        weather_temperature_text: Some("12°"),
        weather_temperature_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 4),
        weather_location_text: Some("Riga"),
        weather_location_style: TextStyle::new(ClearColor::opaque(180, 190, 210), 2),
        weather_icon: Some(veila_renderer::icon::WeatherIcon::Cloudy),
        weather_icon_size: Some(34),
        weather_icon_gap: Some(10),
        weather_location_gap: Some(3),
        weather_icon_opacity: Some(41),
        weather_horizontal_padding: Some(64),
        weather_alignment: WeatherAlignment::Right,
        weather_left_offset: Some(12),
        weather_bottom_offset: Some(-6),
        metrics,
    });

    let weather = blocks.weather.expect("weather blocks");
    assert_eq!(weather.icon_size, 34);
    assert_eq!(weather.icon_gap, 10);
    assert_eq!(weather.location_gap, 3);
    assert_eq!(weather.icon_opacity, Some(41));
    assert_eq!(weather.alignment, WeatherAlignment::Right);
    assert_eq!(weather.horizontal_padding, 64);
    assert_eq!(weather.left_offset, 12);
    assert_eq!(weather.bottom_offset, -6);
}

#[test]
fn text_layout_cache_allows_weather_icon_sizes_above_previous_cap() {
    let mut cache = TextLayoutCache::default();
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);

    let blocks = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_offset_x: None,
        clock_meridiem_offset_y: None,
        date_text: Some("Tuesday"),
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: None,
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: Some("Type your password to unlock"),
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        weather_temperature_text: Some("12°"),
        weather_temperature_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 4),
        weather_location_text: Some("Riga"),
        weather_location_style: TextStyle::new(ClearColor::opaque(180, 190, 210), 2),
        weather_icon: Some(veila_renderer::icon::WeatherIcon::Cloudy),
        weather_icon_size: Some(64),
        weather_icon_gap: None,
        weather_location_gap: None,
        weather_icon_opacity: None,
        weather_horizontal_padding: None,
        weather_alignment: WeatherAlignment::Left,
        weather_left_offset: None,
        weather_bottom_offset: None,
        metrics,
    });

    assert_eq!(blocks.weather.expect("weather blocks").icon_size, 64);
}

#[test]
fn text_layout_cache_reuses_matching_clock_layout() {
    let mut cache = TextLayoutCache::default();
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 5);

    let first = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: style.clone(),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_offset_x: None,
        clock_meridiem_offset_y: None,
        date_text: Some("Tuesday"),
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: Some("ramces"),
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: Some("Type your password to unlock"),
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        weather_temperature_text: None,
        weather_temperature_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        weather_location_text: None,
        weather_location_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 1),
        weather_icon: None,
        weather_icon_size: None,
        weather_icon_gap: None,
        weather_location_gap: None,
        weather_icon_opacity: None,
        weather_horizontal_padding: None,
        weather_alignment: WeatherAlignment::Left,
        weather_left_offset: None,
        weather_bottom_offset: None,
        metrics,
    });
    let cached_clock = cache.clock.block.clone().expect("cached clock block");
    let second = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: style,
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_offset_x: None,
        clock_meridiem_offset_y: None,
        date_text: Some("Tuesday"),
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: Some("ramces"),
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: Some("Type your password to unlock"),
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        weather_temperature_text: None,
        weather_temperature_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        weather_location_text: None,
        weather_location_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 1),
        weather_icon: None,
        weather_icon_size: None,
        weather_icon_gap: None,
        weather_location_gap: None,
        weather_icon_opacity: None,
        weather_horizontal_padding: None,
        weather_alignment: WeatherAlignment::Left,
        weather_left_offset: None,
        weather_bottom_offset: None,
        metrics,
    });

    assert_eq!(first.clock, second.clock);
    assert_eq!(cached_clock, second.clock.expect("clock").primary);
}

#[test]
fn text_layout_cache_refreshes_when_clock_text_changes() {
    let mut cache = TextLayoutCache::default();
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);

    let first = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_offset_x: None,
        clock_meridiem_offset_y: None,
        date_text: Some("Tuesday"),
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: None,
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: Some("Type your password to unlock"),
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        weather_temperature_text: None,
        weather_temperature_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        weather_location_text: None,
        weather_location_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 1),
        weather_icon: None,
        weather_icon_size: None,
        weather_icon_gap: None,
        weather_location_gap: None,
        weather_icon_opacity: None,
        weather_horizontal_padding: None,
        weather_alignment: WeatherAlignment::Left,
        weather_left_offset: None,
        weather_bottom_offset: None,
        metrics,
    });
    let second = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:42"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_offset_x: None,
        clock_meridiem_offset_y: None,
        date_text: Some("Tuesday"),
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: None,
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: Some("Type your password to unlock"),
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        weather_temperature_text: None,
        weather_temperature_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        weather_location_text: None,
        weather_location_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 1),
        weather_icon: None,
        weather_icon_size: None,
        weather_icon_gap: None,
        weather_location_gap: None,
        weather_icon_opacity: None,
        weather_horizontal_padding: None,
        weather_alignment: WeatherAlignment::Left,
        weather_left_offset: None,
        weather_bottom_offset: None,
        metrics,
    });

    assert_ne!(
        first.clock.expect("first clock").primary.lines,
        second.clock.expect("second clock").primary.lines
    );
    assert_eq!(
        cache.clock.key.as_ref().map(|key| key.text.as_str()),
        Some("09:42")
    );
}

#[test]
fn text_layout_cache_builds_stacked_clock_blocks() {
    let mut cache = TextLayoutCache::default();
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);

    let blocks = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Stacked,
        clock_text: Some("06"),
        clock_secondary_text: Some("08"),
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 14),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 3),
        clock_meridiem_offset_x: None,
        clock_meridiem_offset_y: None,
        date_text: None,
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: None,
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: None,
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        weather_temperature_text: None,
        weather_temperature_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        weather_location_text: None,
        weather_location_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 1),
        weather_icon: None,
        weather_icon_size: None,
        weather_icon_gap: None,
        weather_location_gap: None,
        weather_icon_opacity: None,
        weather_horizontal_padding: None,
        weather_alignment: WeatherAlignment::Left,
        weather_left_offset: None,
        weather_bottom_offset: None,
        metrics,
    });

    let clock = blocks.clock.expect("clock blocks");
    assert_eq!(clock.style, ClockStyle::Stacked);
    assert_eq!(clock.primary.lines, vec![String::from("06")]);
    assert_eq!(
        clock.secondary.expect("stacked minute block").lines,
        vec![String::from("08")]
    );
}

#[test]
fn text_layout_cache_reuses_matching_revealed_secret_layout() {
    let mut cache = TextLayoutCache::default();
    let style = TextStyle::new(ClearColor::rgba(240, 244, 250, 236), 2);

    let first = cache.revealed_secret_block("secret", style.clone(), 212);
    let cached = cache
        .revealed_secret
        .block
        .clone()
        .expect("cached revealed secret block");
    let second = cache.revealed_secret_block("secret", style, 212);

    assert_eq!(first, second);
    assert_eq!(cached, second);
}
