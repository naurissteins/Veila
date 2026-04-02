use super::*;

#[test]
#[ignore = "legacy pre-theme defaults"]
fn parses_partial_config_with_defaults() {
    let config = AppConfig::from_toml_str(
        r#"
            [background]
            color = [12, 16, 24]
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.lock.acquire_timeout_seconds, 5);
    assert!(!config.lock.auto_reload_config);
    assert_eq!(config.lock.auto_reload_debounce_ms, 250);
    assert!(config.lock.show_username);
    assert!(config.lock.username.is_none());
    assert!(config.lock.user_hint.is_none());
    assert!(config.lock.avatar_path.is_none());
    assert_eq!(config.background.effective_mode(), BackgroundMode::Bundled);
    assert_eq!(config.background.color, RgbColor::rgb(12, 16, 24));
    assert!(config.background.path.is_none());
    assert!(
        config
            .background
            .resolved_path()
            .is_some_and(|path| path.ends_with("assets/bg/abstract-blur-blue.jpg"))
    );
    assert_eq!(config.background.blur_radius, 0);
    assert_eq!(config.background.dim_strength, 34);
    assert!(config.background.tint.is_none());
    assert_eq!(config.background.tint_opacity, 0);
    assert!(!config.weather.enabled);
    assert!(config.weather.location.is_none());
    assert!(config.weather.clone().coordinates().is_none());
    assert_eq!(config.weather.refresh_minutes, 15);
    assert_eq!(config.weather.unit, WeatherUnit::Celsius);
    assert!(!config.battery.enabled);
    assert_eq!(config.battery.refresh_seconds, 30);
    assert!(config.battery.mock_percent.is_none());
    assert!(config.battery.mock_charging.is_none());
    assert!(matches!(config.visuals.input, InputVisualEntry::Color(_)));
    assert!(config.visuals.input_font_family().is_none());
    assert!(config.visuals.input_font_weight().is_none());
    assert!(config.visuals.input_font_style().is_none());
    assert!(config.visuals.input_font_size().is_none());
    assert_eq!(
        config.visuals.input_alignment(),
        InputAlignment::CenterCenter
    );
    assert!(!config.visuals.input_center_in_layer());
    assert!(config.visuals.input_horizontal_padding().is_none());
    assert!(config.visuals.input_vertical_padding().is_none());
    assert!(config.visuals.input_offset_x().is_none());
    assert!(config.visuals.input_offset_y().is_none());
    assert!(config.visuals.input_background_opacity().is_none());
    assert!(config.visuals.input_border_opacity().is_none());
    assert!(config.visuals.input_width().is_none());
    assert!(config.visuals.input_height().is_none());
    assert_eq!(config.visuals.input_radius(), 32);
    assert!(config.visuals.input_border_width().is_none());
    assert!(config.visuals.avatar_background_color().is_none());
    assert!(config.visuals.avatar_size().is_none());
    assert!(config.visuals.avatar_placeholder_padding().is_none());
    assert!(config.visuals.avatar_icon_color().is_none());
    assert!(config.visuals.avatar_ring_color().is_none());
    assert!(config.visuals.avatar_ring_width().is_none());
    assert!(config.visuals.avatar_background_opacity().is_none());
    assert!(config.visuals.username_color().is_none());
    assert!(config.visuals.username_opacity().is_none());
    assert!(config.visuals.username_size().is_none());
    assert!(config.visuals.avatar_gap().is_none());
    assert!(config.visuals.username_gap().is_none());
    assert!(config.visuals.status_gap().is_none());
    assert!(config.visuals.clock_gap().is_none());
    assert!(config.visuals.auth_stack_offset().is_none());
    assert!(config.visuals.header_top_offset().is_none());
    assert!(config.visuals.identity_gap().is_none());
    assert_eq!(
        config.visuals.center_stack_order(),
        CenterStackOrder::HeroAuth
    );
    assert_eq!(
        config.visuals.center_stack_style(),
        CenterStackStyle::HeroAuth
    );
    assert!(config.visuals.clock_font_family().is_none());
    assert!(config.visuals.clock_font_weight().is_none());
    assert!(config.visuals.clock_font_style().is_none());
    assert_eq!(config.visuals.clock_alignment(), ClockAlignment::TopCenter);
    assert!(!config.visuals.clock_center_in_layer());
    assert_eq!(config.visuals.clock_offset_x(), Some(0));
    assert_eq!(config.visuals.clock_offset_y(), Some(0));
    assert_eq!(config.visuals.clock_format(), ClockFormat::TwentyFourHour);
    assert!(config.visuals.clock_meridiem_size().is_none());
    assert!(config.visuals.clock_meridiem_offset_x().is_none());
    assert!(config.visuals.clock_meridiem_offset_y().is_none());
    assert!(config.visuals.clock_color().is_none());
    assert!(config.visuals.clock_opacity().is_none());
    assert!(config.visuals.date_color().is_none());
    assert!(config.visuals.date_font_family().is_none());
    assert!(config.visuals.date_font_weight().is_none());
    assert!(config.visuals.date_font_style().is_none());
    assert!(config.visuals.date_opacity().is_none());
    assert!(config.visuals.clock_size().is_none());
    assert!(config.visuals.date_size().is_none());
    assert!(config.visuals.placeholder_color().is_none());
    assert!(config.visuals.placeholder_opacity().is_none());
    assert!(config.visuals.eye_icon_color().is_none());
    assert!(config.visuals.eye_icon_opacity().is_none());
    assert!(config.visuals.keyboard_color().is_none());
    assert!(config.visuals.keyboard_background_color().is_none());
    assert!(config.visuals.keyboard_background_size().is_none());
    assert!(config.visuals.keyboard_opacity().is_none());
    assert!(config.visuals.keyboard_size().is_none());
    assert!(config.visuals.keyboard_top_offset().is_none());
    assert!(config.visuals.keyboard_right_offset().is_none());
    assert!(config.visuals.battery_background_color().is_none());
    assert!(config.visuals.battery_color().is_none());
    assert!(config.visuals.battery_background_size().is_none());
    assert!(config.visuals.battery_opacity().is_none());
    assert!(config.visuals.battery_size().is_none());
    assert!(config.visuals.battery_top_offset().is_none());
    assert!(config.visuals.battery_right_offset().is_none());
    assert!(config.visuals.battery_gap().is_none());
    assert!(config.visuals.weather_size().is_none());
    assert!(config.visuals.weather_temperature_color().is_none());
    assert!(config.visuals.weather_location_color().is_none());
    assert!(config.visuals.weather_temperature_font_family().is_none());
    assert!(config.visuals.weather_temperature_font_weight().is_none());
    assert!(config.visuals.weather_temperature_font_style().is_none());
    assert!(config.visuals.weather_location_font_family().is_none());
    assert!(config.visuals.weather_location_font_weight().is_none());
    assert!(config.visuals.weather_location_font_style().is_none());
    assert!(
        config
            .visuals
            .weather_temperature_letter_spacing()
            .is_none()
    );
    assert!(config.visuals.weather_temperature_size().is_none());
    assert!(config.visuals.weather_location_size().is_none());
    assert!(config.visuals.weather_icon_size().is_none());
    assert!(config.visuals.weather_icon_gap().is_none());
    assert!(config.visuals.weather_location_gap().is_none());
    assert_eq!(
        config.visuals.weather_alignment(),
        super::WeatherAlignment::Left
    );
    assert!(config.visuals.weather_horizontal_padding().is_none());
    assert!(config.visuals.weather_left_padding().is_none());
    assert!(config.visuals.weather_bottom_padding().is_none());
    assert!(config.visuals.now_playing_title_color().is_none());
    assert!(config.visuals.now_playing_artist_color().is_none());
    assert!(config.visuals.username_font_family().is_none());
    assert!(config.visuals.username_font_weight().is_none());
    assert!(config.visuals.now_playing_fade_duration_ms().is_none());
    assert!(config.visuals.now_playing_title_font_family().is_none());
    assert!(config.visuals.now_playing_artist_font_family().is_none());
    assert!(config.visuals.now_playing_title_font_weight().is_none());
    assert!(config.visuals.now_playing_artist_font_weight().is_none());
    assert!(config.visuals.now_playing_title_font_style().is_none());
    assert!(config.visuals.now_playing_artist_font_style().is_none());
    assert!(config.visuals.now_playing_opacity().is_none());
    assert!(config.visuals.now_playing_title_opacity().is_none());
    assert!(config.visuals.now_playing_artist_opacity().is_none());
    assert!(config.visuals.now_playing_artwork_opacity().is_none());
    assert!(config.visuals.now_playing_title_size().is_none());
    assert!(config.visuals.now_playing_artist_size().is_none());
    assert!(config.visuals.now_playing_width().is_none());
    assert!(config.visuals.now_playing_content_gap().is_none());
    assert!(config.visuals.now_playing_text_gap().is_none());
    assert!(config.visuals.now_playing_artwork_size().is_none());
    assert!(config.visuals.now_playing_artwork_radius().is_none());
    assert!(config.visuals.now_playing_right_padding().is_none());
    assert!(config.visuals.now_playing_bottom_padding().is_none());
    assert!(config.visuals.now_playing_right_offset().is_none());
    assert!(config.visuals.now_playing_bottom_offset().is_none());
    assert!(config.visuals.status_color().is_none());
    assert!(config.visuals.status_opacity().is_none());
    assert!(config.visuals.input_mask_color().is_none());
}
