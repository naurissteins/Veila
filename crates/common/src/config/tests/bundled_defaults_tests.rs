use super::*;

#[test]
fn first_run_defaults_match_bundled_theme() {
    let config = AppConfig::default();

    assert_eq!(config.lock.acquire_timeout_seconds, 5);
    assert!(config.lock.auto_reload_config);
    assert_eq!(config.lock.auto_reload_debounce_ms, 250);
    assert!(!config.lock.log_to_file);
    assert_eq!(
        config.lock.log_file_path,
        std::path::PathBuf::from("~/.local/state/veila/veilad.log")
    );
    assert!(config.lock.show_username);
    assert!(config.lock.username.is_none());
    assert_eq!(config.lock.user_hint.as_deref(), Some("Password"));
    assert!(config.lock.avatar_path.is_none());
    assert_eq!(config.background.effective_mode(), BackgroundMode::Bundled);
    assert_eq!(config.background.color, RgbColor::rgb(32, 40, 51));
    assert!(config.background.resolved_path().is_some_and(|path| {
        path.ends_with("assets/bg/default.jpg") || path.ends_with("assets/bg/bg.jpg")
    }));
    assert_eq!(config.background.blur_radius, 12);
    assert_eq!(config.background.dim_strength, 54);
    assert_eq!(config.background.tint, Some(RgbColor::rgba(8, 10, 14, 102)));
    assert_eq!(config.background.tint_opacity, 0);
    assert!(config.weather.enabled);
    assert_eq!(config.weather.location.as_deref(), Some("Riga"));
    assert!(config.weather.clone().coordinates().is_none());
    assert_eq!(config.weather.refresh_minutes, 15);
    assert_eq!(config.weather.unit, WeatherUnit::Celsius);
    assert!(config.battery.enabled);
    assert_eq!(config.battery.refresh_seconds, 30);
    assert!(config.battery.mock_percent.is_none());
    assert!(config.battery.mock_charging.is_none());
    assert!(matches!(config.visuals.input, InputVisualEntry::Section(_)));
    assert_eq!(config.visuals.input_font_family(), Some("Google Sans Flex"));
    assert_eq!(config.visuals.input_font_weight(), Some(400));
    assert_eq!(config.visuals.input_font_style(), Some(FontStyle::Normal));
    assert_eq!(config.visuals.input_font_size(), Some(2));
    assert_eq!(
        config.visuals.input_alignment(),
        InputAlignment::CenterCenter
    );
    assert!(!config.visuals.input_center_in_layer());
    assert!(config.visuals.input_horizontal_padding().is_none());
    assert!(config.visuals.input_vertical_padding().is_none());
    assert_eq!(config.visuals.input_offset_x(), Some(0));
    assert_eq!(config.visuals.input_offset_y(), Some(0));
    assert_eq!(
        config.visuals.input_background_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(config.visuals.input_background_opacity(), Some(5));
    assert_eq!(
        config.visuals.input_border_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(config.visuals.input_border_opacity(), Some(0));
    assert_eq!(config.visuals.input_width(), Some(310));
    assert_eq!(config.visuals.input_height(), Some(54));
    assert_eq!(config.visuals.input_radius(), 10);
    assert_eq!(config.visuals.input_border_width(), Some(0));
    assert_eq!(
        config.visuals.avatar_background_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.avatar_size(), Some(192));
    assert_eq!(config.visuals.avatar_offset_y(), Some(0));
    assert_eq!(config.visuals.avatar_placeholder_padding(), Some(28));
    assert_eq!(
        config.visuals.avatar_icon_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(
        config.visuals.avatar_ring_color(),
        Some(RgbColor::rgb(148, 178, 255))
    );
    assert_eq!(config.visuals.avatar_ring_width(), Some(0));
    assert_eq!(config.visuals.avatar_background_opacity(), Some(6));
    assert_eq!(
        config.visuals.username_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(
        config.visuals.username_font_family(),
        Some("Google Sans Flex")
    );
    assert_eq!(config.visuals.username_font_weight(), Some(400));
    assert_eq!(
        config.visuals.username_font_style(),
        Some(FontStyle::Normal)
    );
    assert_eq!(config.visuals.username_opacity(), Some(84));
    assert_eq!(config.visuals.username_size(), Some(4));
    assert_eq!(config.visuals.username_offset_y(), Some(0));
    assert_eq!(config.visuals.avatar_gap(), Some(24));
    assert_eq!(config.visuals.username_gap(), Some(28));
    assert_eq!(config.visuals.status_gap(), Some(18));
    assert_eq!(config.visuals.clock_gap(), Some(20));
    assert_eq!(config.visuals.auth_stack_offset(), Some(0));
    assert_eq!(config.visuals.header_top_offset(), Some(-12));
    assert_eq!(config.visuals.identity_gap(), Some(18));
    assert_eq!(
        config.visuals.center_stack_order(),
        CenterStackOrder::HeroAuth
    );
    assert_eq!(
        config.visuals.center_stack_style(),
        CenterStackStyle::HeroAuth
    );
    assert_eq!(config.visuals.clock_font_family(), Some("Geom"));
    assert_eq!(config.visuals.clock_font_weight(), Some(600));
    assert_eq!(config.visuals.clock_font_style(), Some(FontStyle::Normal));
    assert_eq!(config.visuals.clock_style(), ClockStyle::Standard);
    assert_eq!(config.visuals.clock_alignment(), ClockAlignment::TopCenter);
    assert!(!config.visuals.clock_center_in_layer());
    assert_eq!(config.visuals.clock_offset_x(), Some(0));
    assert_eq!(config.visuals.clock_offset_y(), Some(0));
    assert_eq!(config.visuals.clock_format(), ClockFormat::TwentyFourHour);
    assert_eq!(config.visuals.clock_meridiem_size(), Some(3));
    assert_eq!(config.visuals.clock_meridiem_offset_x(), Some(6));
    assert_eq!(config.visuals.clock_meridiem_offset_y(), Some(7));
    assert_eq!(
        config.visuals.clock_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.clock_opacity(), Some(40));
    assert_eq!(config.visuals.clock_size(), Some(14));
    assert_eq!(
        config.visuals.date_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.date_font_family(), Some("Geom"));
    assert_eq!(config.visuals.date_font_weight(), Some(600));
    assert_eq!(config.visuals.date_font_style(), Some(FontStyle::Normal));
    assert_eq!(config.visuals.date_opacity(), Some(50));
    assert_eq!(config.visuals.date_size(), Some(2));
    assert_eq!(
        config.visuals.placeholder_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.placeholder_opacity(), Some(60));
    assert_eq!(
        config.visuals.status_color(),
        Some(RgbColor::rgb(255, 224, 160))
    );
    assert_eq!(config.visuals.status_opacity(), Some(88));
    assert_eq!(
        config.visuals.eye_icon_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.eye_icon_opacity(), Some(72));
    assert_eq!(
        config.visuals.keyboard_background_color(),
        Some(RgbColor::rgba(255, 255, 255, 13))
    );
    assert_eq!(
        config.visuals.keyboard_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.keyboard_background_size(), Some(46));
    assert_eq!(config.visuals.keyboard_opacity(), Some(68));
    assert_eq!(config.visuals.keyboard_size(), Some(2));
    assert_eq!(config.visuals.keyboard_top_offset(), Some(-24));
    assert_eq!(config.visuals.keyboard_right_offset(), Some(8));
    assert_eq!(
        config.visuals.battery_background_color(),
        Some(RgbColor::rgba(255, 255, 255, 13))
    );
    assert_eq!(
        config.visuals.battery_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.battery_background_size(), Some(46));
    assert_eq!(config.visuals.battery_opacity(), Some(68));
    assert_eq!(config.visuals.battery_size(), Some(20));
    assert_eq!(config.visuals.battery_top_offset(), Some(-24));
    assert_eq!(config.visuals.battery_right_offset(), Some(8));
    assert_eq!(config.visuals.battery_gap(), Some(8));
    assert!(!config.visuals.layer_enabled());
    assert_eq!(config.visuals.layer_mode(), LayerMode::Blur);
    assert_eq!(config.visuals.layer_style(), LayerStyle::Panel);
    assert!(!config.visuals.layer_full_width());
    assert_eq!(config.visuals.layer_alignment(), LayerAlignment::Center);
    assert_eq!(config.visuals.layer_width(), Some(560));
    assert!(config.visuals.layer_full_height());
    assert_eq!(config.visuals.layer_height(), None);
    assert_eq!(
        config.visuals.layer_vertical_alignment(),
        LayerVerticalAlignment::Top
    );
    assert_eq!(config.visuals.layer_offset_x(), Some(0));
    assert_eq!(config.visuals.layer_offset_y(), Some(0));
    assert_eq!(config.visuals.layer_left_padding(), Some(0));
    assert_eq!(config.visuals.layer_right_padding(), Some(0));
    assert_eq!(config.visuals.layer_top_padding(), Some(0));
    assert_eq!(config.visuals.layer_bottom_padding(), Some(0));
    assert_eq!(config.visuals.layer_color(), Some(RgbColor::rgb(8, 10, 14)));
    assert_eq!(config.visuals.layer_opacity(), Some(42));
    assert_eq!(config.visuals.layer_blur_radius(), Some(12));
    assert_eq!(config.visuals.layer_radius(), Some(0));
    assert_eq!(
        config.visuals.layer_border_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.layer_border_width(), Some(0));
    assert_eq!(config.visuals.weather_size(), Some(2));
    assert_eq!(config.visuals.weather_opacity(), Some(50));
    assert_eq!(
        config.visuals.weather_temperature_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(
        config.visuals.weather_location_color(),
        Some(RgbColor::rgb(214, 227, 255))
    );
    assert_eq!(
        config.visuals.weather_temperature_font_family(),
        Some("Geom")
    );
    assert_eq!(config.visuals.weather_temperature_font_weight(), Some(600));
    assert_eq!(
        config.visuals.weather_temperature_font_style(),
        Some(FontStyle::Normal)
    );
    assert_eq!(config.visuals.weather_temperature_letter_spacing(), Some(0));
    assert_eq!(
        config.visuals.weather_location_font_family(),
        Some("Google Sans Flex")
    );
    assert_eq!(config.visuals.weather_location_font_weight(), Some(400));
    assert_eq!(
        config.visuals.weather_location_font_style(),
        Some(FontStyle::Normal)
    );
    assert_eq!(config.visuals.weather_temperature_size(), Some(6));
    assert_eq!(config.visuals.weather_location_size(), Some(3));
    assert_eq!(config.visuals.weather_icon_size(), Some(40));
    assert_eq!(config.visuals.weather_icon_gap(), Some(1));
    assert_eq!(config.visuals.weather_location_gap(), Some(1));
    assert_eq!(
        config.visuals.weather_alignment(),
        super::WeatherAlignment::Left
    );
    assert_eq!(config.visuals.weather_left_offset(), Some(12));
    assert_eq!(config.visuals.weather_bottom_offset(), Some(-6));
    assert_eq!(config.visuals.weather_horizontal_padding(), Some(48));
    assert_eq!(config.visuals.weather_left_padding(), Some(48));
    assert_eq!(config.visuals.weather_bottom_padding(), Some(48));
    assert_eq!(config.visuals.now_playing_fade_duration_ms(), Some(320));
    assert_eq!(config.visuals.now_playing_opacity(), Some(72));
    assert_eq!(config.visuals.now_playing_title_opacity(), Some(74));
    assert_eq!(config.visuals.now_playing_artist_opacity(), Some(54));
    assert_eq!(config.visuals.now_playing_artwork_opacity(), Some(90));
    assert_eq!(
        config.visuals.now_playing_title_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(
        config.visuals.now_playing_artist_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(
        config.visuals.now_playing_title_font_family(),
        Some("Google Sans Flex")
    );
    assert_eq!(
        config.visuals.now_playing_artist_font_family(),
        Some("Google Sans Flex")
    );
    assert_eq!(config.visuals.now_playing_title_font_weight(), Some(400));
    assert_eq!(config.visuals.now_playing_artist_font_weight(), Some(400));
    assert_eq!(
        config.visuals.now_playing_title_font_style(),
        Some(FontStyle::Normal)
    );
    assert_eq!(
        config.visuals.now_playing_artist_font_style(),
        Some(FontStyle::Normal)
    );
    assert_eq!(config.visuals.now_playing_title_size(), Some(2));
    assert_eq!(config.visuals.now_playing_artist_size(), Some(2));
    assert_eq!(config.visuals.now_playing_width(), Some(380));
    assert_eq!(config.visuals.now_playing_content_gap(), Some(18));
    assert_eq!(config.visuals.now_playing_text_gap(), Some(10));
    assert_eq!(config.visuals.now_playing_artwork_size(), Some(44));
    assert_eq!(config.visuals.now_playing_artwork_radius(), Some(8));
    assert_eq!(config.visuals.now_playing_right_padding(), Some(52));
    assert_eq!(config.visuals.now_playing_bottom_padding(), Some(56));
    assert_eq!(config.visuals.now_playing_right_offset(), Some(0));
    assert_eq!(config.visuals.now_playing_bottom_offset(), Some(0));
    assert_eq!(
        config.visuals.input_mask_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
}
