use super::*;
use crate::RevealDisplayMode;

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
    assert!(config.lock.auto_reload_config);
    assert_eq!(config.lock.auto_reload_debounce_ms, 250);
    assert!(!config.lock.log_to_file);
    assert_eq!(
        config.lock.log_file_path,
        std::path::PathBuf::from("~/.local/state/veila/veilad.log")
    );
    assert!(config.lock.avatar_path.is_none());
    assert_eq!(config.background.effective_mode(), BackgroundMode::Gradient);
    assert_eq!(config.background.color, RgbColor::rgb(12, 16, 24));
    assert!(config.background.path.is_none());
    assert!(config.background.resolved_path().is_none());
    let gradient = config
        .background
        .resolved_gradient()
        .expect("gradient defaults should resolve");
    assert_eq!(gradient.top_left, RgbColor::rgb(168, 91, 255));
    assert_eq!(gradient.top_right, RgbColor::rgb(57, 184, 255));
    assert_eq!(gradient.bottom_left, RgbColor::rgb(111, 226, 255));
    assert_eq!(gradient.bottom_right, RgbColor::rgb(111, 76, 255));
    let radial = config
        .background
        .radial
        .as_ref()
        .expect("radial defaults should exist");
    assert_eq!(radial.center, RgbColor::rgb(111, 226, 255));
    assert_eq!(radial.edge, RgbColor::rgb(111, 76, 255));
    assert_eq!(radial.center_x, 50);
    assert_eq!(radial.center_y, 50);
    assert_eq!(radial.radius, 100);
    let layered = config
        .background
        .layered
        .as_ref()
        .expect("layered defaults should exist");
    assert_eq!(
        layered.base.effective_mode(),
        crate::config::BackgroundLayeredBaseMode::Gradient
    );
    assert!(layered.blobs.is_empty());
    assert_eq!(config.background.blur_strength, 0);
    assert_eq!(config.background.dim_strength, 34);
    assert!(config.background.tint.is_none());
    assert!(!config.lock.power_off_secondary_outputs);
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
    assert!(!config.visuals.input_reveal_on_interaction());
    assert_eq!(config.visuals.input_reveal_mode(), InputRevealMode::Input);
    assert_eq!(
        config.visuals.input_reveal_hint(),
        "Press any key or click to continue"
    );
    assert_eq!(config.visuals.reveal_mode(), RevealDisplayMode::Shown);
    assert!(config.visuals.reveal_enabled());
    assert_eq!(
        config.visuals.reveal_text(),
        "Press any key or click to continue"
    );
    assert!(config.visuals.reveal_color().is_none());
    assert!(config.visuals.reveal_font_family().is_none());
    assert!(config.visuals.reveal_font_weight().is_none());
    assert!(config.visuals.reveal_font_style().is_none());
    assert!(config.visuals.reveal_font_size().is_none());
    assert!(config.visuals.input_width().is_none());
    assert!(config.visuals.input_height().is_none());
    assert_eq!(config.visuals.input_radius(), 32);
    assert!(config.visuals.input_border_width().is_none());
    assert_eq!(config.visuals.input_placeholder(), "Password");
    assert!(config.visuals.avatar_background_color().is_none());
    assert!(config.visuals.avatar_image_path().is_none());
    assert!(config.avatar_image_path().is_none());
    assert!(config.visuals.avatar_size().is_none());
    assert!(config.visuals.avatar_radius().is_none());
    assert!(config.visuals.avatar_placeholder_padding().is_none());
    assert!(config.visuals.avatar_icon_color().is_none());
    assert!(config.visuals.avatar_ring_color().is_none());
    assert!(config.visuals.avatar_ring_width().is_none());
    assert!(config.visuals.username_enabled());
    assert!(config.visuals.username_text().is_none());
    assert!(config.visuals.username_color().is_none());
    assert!(config.visuals.username_font_size().is_none());
    assert!(config.visuals.clock_font_family().is_none());
    assert!(config.visuals.clock_font_weight().is_none());
    assert!(config.visuals.clock_font_style().is_none());
    assert_eq!(config.visuals.clock_format(), ClockFormat::TwentyFourHour);
    assert!(config.visuals.clock_meridiem_font_size().is_none());
    assert!(config.visuals.clock_meridiem_x().is_none());
    assert!(config.visuals.clock_meridiem_y().is_none());
    assert!(config.visuals.clock_color().is_none());
    assert!(config.visuals.date_color().is_none());
    assert_eq!(config.visuals.date_format(), DateFormat::Long);
    assert!(config.visuals.date_font_family().is_none());
    assert!(config.visuals.date_font_weight().is_none());
    assert!(config.visuals.date_font_style().is_none());
    assert!(config.visuals.clock_font_size().is_none());
    assert!(config.visuals.date_font_size().is_none());
    assert!(config.visuals.placeholder_color().is_none());
    assert!(config.visuals.eye_icon_color().is_none());
    assert!(config.visuals.keyboard_color().is_none());
    assert!(config.visuals.keyboard_background_color().is_none());
    assert!(config.visuals.keyboard_background_size().is_none());
    assert!(config.visuals.keyboard_size().is_none());
    assert_eq!(
        config.visuals.keyboard_position(),
        WidgetPositionConfig::default()
    );
    assert!(config.visuals.battery_background_color().is_none());
    assert!(config.visuals.battery_color().is_none());
    assert!(config.visuals.battery_background_size().is_none());
    assert!(config.visuals.battery_size().is_none());
    assert_eq!(
        config.visuals.battery_position(),
        WidgetPositionConfig::default()
    );
    assert_eq!(
        config.visuals.power_status_position(),
        WidgetPositionConfig::default()
    );
    assert!(config.visuals.weather_icon_enabled());
    assert!(config.visuals.weather_temperature_enabled());
    assert!(config.visuals.weather_location_enabled());
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
    assert!(config.visuals.weather_temperature_font_size().is_none());
    assert!(config.visuals.weather_location_font_size().is_none());
    assert!(config.visuals.weather_icon_size().is_none());
    assert_eq!(
        config.visuals.weather_icon_position(),
        WidgetPositionConfig::default()
    );
    assert_eq!(
        config.visuals.weather_temperature_position(),
        WidgetPositionConfig::default()
    );
    assert_eq!(
        config.visuals.weather_location_position(),
        WidgetPositionConfig::default()
    );
    assert!(config.visuals.now_playing_title_color().is_none());
    assert!(config.visuals.now_playing_artist_color().is_none());
    assert!(config.visuals.now_playing_artist_enabled());
    assert!(config.visuals.now_playing_title_enabled());
    assert!(config.visuals.username_font_family().is_none());
    assert!(config.visuals.username_font_weight().is_none());
    assert!(config.visuals.now_playing_fade_duration_ms().is_none());
    assert!(config.visuals.now_playing_title_font_family().is_none());
    assert!(config.visuals.now_playing_artist_font_family().is_none());
    assert!(config.visuals.now_playing_title_font_weight().is_none());
    assert!(config.visuals.now_playing_artist_font_weight().is_none());
    assert!(config.visuals.now_playing_title_font_style().is_none());
    assert!(config.visuals.now_playing_artist_font_style().is_none());
    assert!(config.visuals.now_playing_artwork_size().is_none());
    assert!(config.visuals.now_playing_artwork_radius().is_none());
    assert!(config.visuals.now_playing_artwork_opacity().is_none());
    assert!(config.visuals.now_playing_title_font_size().is_none());
    assert!(config.visuals.now_playing_artist_font_size().is_none());
    assert!(config.visuals.now_playing_title_width().is_none());
    assert!(config.visuals.now_playing_artist_width().is_none());
    assert_eq!(
        config.visuals.now_playing_artwork_position(),
        WidgetPositionConfig::default()
    );
    assert_eq!(
        config.visuals.now_playing_artist_position(),
        WidgetPositionConfig::default()
    );
    assert_eq!(
        config.visuals.now_playing_title_position(),
        WidgetPositionConfig::default()
    );
    assert!(config.visuals.status_color().is_none());
    assert!(config.visuals.input_mask_color().is_none());
}
