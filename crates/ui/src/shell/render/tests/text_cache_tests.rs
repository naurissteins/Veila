use super::*;

#[test]
fn text_layout_cache_uses_configured_weather_icon_size() {
    let mut cache = TextLayoutCache::default();
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    let blocks = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_x: None,
        clock_meridiem_y: None,
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
        weather_icon_opacity: Some(41),
        metrics,
    });

    let weather = blocks.weather.expect("weather blocks");
    assert_eq!(
        weather
            .temperature
            .as_ref()
            .map(|block| block.lines[0].as_str()),
        Some("12°")
    );
    assert_eq!(
        weather
            .location
            .as_ref()
            .map(|block| block.lines[0].as_str()),
        Some("Riga")
    );
    assert_eq!(
        weather.icon,
        Some(super::super::model::SceneWeatherIcon {
            asset: veila_renderer::icon::WeatherIcon::Cloudy,
            size: 34,
            opacity: Some(41),
        })
    );
}

#[test]
fn text_layout_cache_allows_weather_icon_sizes_above_previous_cap() {
    let mut cache = TextLayoutCache::default();
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    let blocks = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_x: None,
        clock_meridiem_y: None,
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
        weather_icon_opacity: None,
        metrics,
    });

    assert_eq!(
        blocks
            .weather
            .expect("weather blocks")
            .icon
            .map(|icon| icon.size),
        Some(64)
    );
}

#[test]
fn text_layout_cache_reuses_matching_clock_layout() {
    let mut cache = TextLayoutCache::default();
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 5);

    let first = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: style.clone(),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_x: None,
        clock_meridiem_y: None,
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
        weather_icon_opacity: None,
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
        clock_meridiem_x: None,
        clock_meridiem_y: None,
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
        weather_icon_opacity: None,
        metrics,
    });

    assert_eq!(first.clock, second.clock);
    assert_eq!(cached_clock, second.clock.expect("clock").primary);
}

#[test]
fn text_layout_cache_refreshes_when_clock_text_changes() {
    let mut cache = TextLayoutCache::default();
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    let first = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_x: None,
        clock_meridiem_y: None,
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
        weather_icon_opacity: None,
        metrics,
    });
    let second = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:42"),
        clock_secondary_text: None,
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        clock_meridiem_x: None,
        clock_meridiem_y: None,
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
        weather_icon_opacity: None,
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
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    let blocks = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Stacked,
        clock_text: Some("06"),
        clock_secondary_text: Some("08"),
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 14),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 3),
        clock_meridiem_x: None,
        clock_meridiem_y: None,
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
        weather_icon_opacity: None,
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
fn large_standard_clock_stays_single_line() {
    let mut cache = TextLayoutCache::default();
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    let blocks = cache.scene_text_blocks(SceneTextInputs {
        clock_style_mode: ClockStyle::Standard,
        clock_text: Some("09:41"),
        clock_secondary_text: None,
        clock_style: TextStyle::new_px(ClearColor::opaque(255, 255, 255), 288),
        clock_meridiem_text: None,
        clock_meridiem_style: TextStyle::new_px(ClearColor::opaque(255, 255, 255), 72),
        clock_meridiem_x: None,
        clock_meridiem_y: None,
        date_text: Some("Tuesday"),
        date_style: TextStyle::new_px(ClearColor::opaque(255, 255, 255), 128),
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
        weather_icon_opacity: None,
        metrics,
    });

    let clock = blocks.clock.expect("clock blocks");
    let date = blocks.date.expect("date block");
    assert_eq!(clock.primary.lines.len(), 1);
    assert_eq!(date.lines.len(), 1);
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
