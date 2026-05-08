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
    assert_eq!(config.background.effective_mode(), BackgroundMode::Gradient);
    assert_eq!(config.background.color, RgbColor::rgb(32, 40, 51));
    let gradient = config
        .background
        .resolved_gradient()
        .expect("gradient defaults should resolve");
    assert_eq!(gradient.top_left, RgbColor::rgb(168, 91, 255));
    assert_eq!(gradient.top_right, RgbColor::rgb(57, 184, 255));
    assert_eq!(gradient.bottom_left, RgbColor::rgb(111, 226, 255));
    assert_eq!(gradient.bottom_right, RgbColor::rgb(111, 76, 255));
    assert!(config.background.resolved_path().is_none());
    assert_eq!(config.background.blur_radius, 0);
    assert_eq!(config.background.dim_strength, 54);
    assert!(config.background.tint.is_none());
    assert!(!config.lock.suspend_only_on_battery);
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
    assert!(!config.visuals.input_reveal_on_interaction());
    assert_eq!(config.visuals.input_reveal_mode(), InputRevealMode::Input);
    assert_eq!(
        config.visuals.input_reveal_hint(),
        "Press any key or click to continue"
    );
    assert!(config.visuals.reveal_enabled());
    assert_eq!(
        config.visuals.reveal_text(),
        "Press any key or click to continue"
    );
    assert_eq!(
        config.visuals.input_background_color(),
        RgbColor::rgba(255, 255, 255, 13)
    );
    assert_eq!(
        config.visuals.input_border_color(),
        RgbColor::rgba(255, 255, 255, 0)
    );
    assert_eq!(config.visuals.input_width(), Some(310));
    assert_eq!(config.visuals.input_height(), Some(54));
    assert_eq!(config.visuals.input_radius(), 10);
    assert_eq!(config.visuals.input_border_width(), Some(0));
    assert_eq!(
        config.visuals.avatar_background_color(),
        Some(RgbColor::rgba(255, 255, 255, 15))
    );
    assert_eq!(config.visuals.avatar_size(), Some(192));
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
    assert_eq!(
        config.visuals.username_color(),
        Some(RgbColor::rgba(255, 255, 255, 214))
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
    assert_eq!(config.visuals.username_size(), Some(4));
    assert_eq!(config.visuals.clock_font_family(), Some("Geom"));
    assert_eq!(config.visuals.clock_font_weight(), Some(600));
    assert_eq!(config.visuals.clock_font_style(), Some(FontStyle::Normal));
    assert_eq!(config.visuals.clock_style(), ClockStyle::Standard);
    assert_eq!(config.visuals.clock_format(), ClockFormat::TwentyFourHour);
    assert_eq!(config.visuals.clock_meridiem_size(), Some(3));
    assert_eq!(config.visuals.clock_meridiem_offset_x(), Some(6));
    assert_eq!(config.visuals.clock_meridiem_offset_y(), Some(7));
    assert_eq!(
        config.visuals.clock_color(),
        Some(RgbColor::rgba(255, 255, 255, 102))
    );
    assert_eq!(config.visuals.clock_size(), Some(14));
    assert_eq!(
        config.visuals.date_color(),
        Some(RgbColor::rgba(255, 255, 255, 128))
    );
    assert_eq!(config.visuals.date_font_family(), Some("Geom"));
    assert_eq!(config.visuals.date_font_weight(), Some(600));
    assert_eq!(config.visuals.date_font_style(), Some(FontStyle::Normal));
    assert_eq!(config.visuals.date_size(), Some(2));
    assert_eq!(
        config.visuals.placeholder_color(),
        Some(RgbColor::rgba(255, 255, 255, 153))
    );
    assert_eq!(
        config.visuals.status_color(),
        Some(RgbColor::rgba(255, 224, 160, 224))
    );
    assert_eq!(
        config.visuals.eye_icon_color(),
        Some(RgbColor::rgba(255, 255, 255, 184))
    );
    assert_eq!(
        config.visuals.keyboard_background_color(),
        Some(RgbColor::rgba(255, 255, 255, 13))
    );
    assert_eq!(
        config.visuals.keyboard_color(),
        Some(RgbColor::rgba(255, 255, 255, 173))
    );
    assert_eq!(config.visuals.keyboard_background_size(), Some(46));
    assert_eq!(config.visuals.keyboard_size(), Some(2));
    assert_eq!(
        config.visuals.keyboard_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Top),
            x: Some(-24),
            y: Some(17),
        }
    );
    assert_eq!(
        config.visuals.battery_background_color(),
        Some(RgbColor::rgba(255, 255, 255, 13))
    );
    assert_eq!(
        config.visuals.battery_color(),
        Some(RgbColor::rgba(255, 255, 255, 173))
    );
    assert_eq!(config.visuals.battery_background_size(), Some(46));
    assert_eq!(config.visuals.battery_size(), Some(20));
    assert_eq!(
        config.visuals.battery_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Top),
            x: Some(-78),
            y: Some(17),
        }
    );
    assert_eq!(
        config.visuals.power_status_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Top),
            x: Some(-24),
            y: Some(17),
        }
    );
    assert!(config.visuals.backdrop.is_empty());
    assert!(config.visuals.weather_icon_enabled());
    assert!(config.visuals.weather_temperature_enabled());
    assert!(config.visuals.weather_location_enabled());
    assert_eq!(config.visuals.weather_icon_opacity(), Some(50));
    assert_eq!(
        config.visuals.weather_temperature_color(),
        Some(RgbColor::rgba(255, 255, 255, 116))
    );
    assert_eq!(
        config.visuals.weather_location_color(),
        Some(RgbColor::rgba(214, 227, 255, 92))
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
    assert_eq!(config.visuals.weather_temperature_font_size(), Some(6));
    assert_eq!(config.visuals.weather_location_font_size(), Some(3));
    assert_eq!(config.visuals.weather_icon_size(), Some(40));
    assert_eq!(
        config.visuals.weather_icon_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Left),
            valign: Some(VerticalAlign::Bottom),
            x: Some(30),
            y: Some(-112),
        }
    );
    assert_eq!(
        config.visuals.weather_temperature_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Left),
            valign: Some(VerticalAlign::Bottom),
            x: Some(30),
            y: Some(-66),
        }
    );
    assert_eq!(
        config.visuals.weather_location_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Left),
            valign: Some(VerticalAlign::Bottom),
            x: Some(30),
            y: Some(-34),
        }
    );
    assert_eq!(config.visuals.now_playing_fade_duration_ms(), Some(320));
    assert_eq!(config.visuals.now_playing_artwork_opacity(), Some(90));
    assert!(config.visuals.now_playing_artwork_enabled());
    assert!(config.visuals.now_playing_artist_enabled());
    assert!(config.visuals.now_playing_title_enabled());
    assert_eq!(
        config.visuals.now_playing_title_color(),
        Some(RgbColor::rgba(255, 255, 255, 175))
    );
    assert_eq!(
        config.visuals.now_playing_artist_color(),
        Some(RgbColor::rgba(255, 255, 255, 99))
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
    assert_eq!(config.visuals.now_playing_title_font_size(), Some(2));
    assert_eq!(config.visuals.now_playing_artist_font_size(), Some(2));
    assert_eq!(config.visuals.now_playing_title_width(), Some(318));
    assert_eq!(config.visuals.now_playing_artist_width(), Some(318));
    assert_eq!(config.visuals.now_playing_artwork_size(), Some(44));
    assert_eq!(config.visuals.now_playing_artwork_radius(), Some(8));
    assert_eq!(
        config.visuals.now_playing_artwork_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Bottom),
            x: Some(-388),
            y: Some(-56),
        }
    );
    assert_eq!(
        config.visuals.now_playing_artist_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Bottom),
            x: Some(-52),
            y: Some(-88),
        }
    );
    assert_eq!(
        config.visuals.now_playing_title_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Bottom),
            x: Some(-52),
            y: Some(-56),
        }
    );
    assert_eq!(
        config.visuals.input_mask_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
}
