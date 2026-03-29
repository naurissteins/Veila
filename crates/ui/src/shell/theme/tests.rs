use veila_common::{
    AppConfig, AvatarVisualConfig, BatteryVisualConfig, ClockAlignment, ClockFormat, ClockStyle,
    ClockVisualConfig, ConfigColor, DateVisualConfig, EyeVisualConfig, FontStyle,
    InputVisualConfig, InputVisualEntry, KeyboardVisualConfig, LayerAlignment, LayerMode,
    LayerVisualConfig, LayoutVisualConfig, NowPlayingVisualConfig, PaletteVisualConfig,
    PlaceholderVisualConfig, StatusVisualConfig, UsernameVisualConfig, WeatherAlignment,
    WeatherVisualConfig,
};
use veila_renderer::ClearColor;

use super::ShellTheme;

#[test]
fn input_opacity_overrides_embedded_alpha() {
    let mut config = AppConfig::default();
    config.visuals.input = InputVisualEntry::Section(InputVisualConfig {
        alignment: Some(veila_common::InputAlignment::CenterCenter),
        horizontal_padding: Some(56),
        vertical_padding: Some(44),
        offset_x: Some(18),
        offset_y: Some(-12),
        font_family: Some(String::from("Geom")),
        font_weight: Some(600),
        font_style: Some(FontStyle::Italic),
        font_size: Some(3),
        background_color: Some(ConfigColor::rgba(255, 255, 255, 200)),
        background_opacity: Some(10),
        border_color: Some(ConfigColor::rgba(255, 255, 255, 180)),
        border_opacity: Some(12),
        width: Some(280),
        height: Some(54),
        radius: None,
        border_width: Some(3),
        mask_color: Some(ConfigColor::rgb(169, 196, 255)),
    });
    config.visuals.avatar = Some(AvatarVisualConfig {
        enabled: Some(true),
        size: Some(92),
        gap: Some(14),
        background_color: Some(ConfigColor::rgb(24, 30, 42)),
        background_opacity: Some(36),
        placeholder_padding: Some(14),
        ring_color: Some(ConfigColor::rgb(148, 178, 255)),
        ring_width: Some(3),
        icon_color: Some(ConfigColor::rgb(232, 238, 249)),
    });
    config.visuals.username = Some(UsernameVisualConfig {
        enabled: Some(true),
        font_family: Some(String::from("Geom")),
        font_weight: Some(600),
        font_style: Some(FontStyle::Italic),
        color: Some(ConfigColor::rgb(215, 227, 255)),
        opacity: Some(72),
        size: Some(3),
        gap: Some(28),
    });
    config.visuals.clock = Some(ClockVisualConfig {
        enabled: Some(true),
        font_family: Some(String::from("Bebas Neue")),
        font_weight: Some(700),
        font_style: Some(FontStyle::Italic),
        style: Some(ClockStyle::Stacked),
        alignment: Some(ClockAlignment::TopLeft),
        offset_x: Some(24),
        offset_y: Some(-16),
        format: Some(ClockFormat::TwelveHour),
        meridiem_size: Some(3),
        meridiem_offset_x: Some(6),
        meridiem_offset_y: Some(-2),
        color: Some(ConfigColor::rgb(248, 251, 255)),
        opacity: Some(96),
        size: Some(4),
        gap: Some(10),
    });
    config.visuals.date = Some(DateVisualConfig {
        enabled: Some(true),
        font_family: Some(String::from("Geom")),
        font_weight: Some(600),
        font_style: Some(FontStyle::Italic),
        color: Some(ConfigColor::rgb(200, 212, 236)),
        opacity: Some(74),
        size: Some(3),
    });
    config.visuals.placeholder = Some(PlaceholderVisualConfig {
        enabled: Some(true),
        color: Some(ConfigColor::rgb(134, 148, 180)),
        opacity: Some(60),
    });
    config.visuals.eye = Some(EyeVisualConfig {
        enabled: Some(true),
        color: Some(ConfigColor::rgb(244, 248, 255)),
        opacity: Some(72),
    });
    config.visuals.keyboard = Some(KeyboardVisualConfig {
        enabled: Some(true),
        background_color: Some(ConfigColor::rgba(18, 22, 30, 82)),
        background_size: Some(42),
        color: Some(ConfigColor::rgb(232, 238, 249)),
        opacity: Some(68),
        size: Some(3),
        top_offset: Some(-12),
        right_offset: Some(8),
    });
    config.visuals.battery = Some(BatteryVisualConfig {
        enabled: Some(true),
        background_color: Some(ConfigColor::rgba(18, 22, 30, 82)),
        background_size: Some(42),
        color: Some(ConfigColor::rgb(255, 255, 255)),
        opacity: Some(72),
        size: Some(18),
        top_offset: Some(-12),
        right_offset: Some(0),
        gap: Some(8),
    });
    config.visuals.layer = Some(LayerVisualConfig {
        enabled: Some(true),
        mode: Some(LayerMode::Blur),
        alignment: Some(LayerAlignment::Right),
        width: Some(520),
        offset_x: Some(-12),
        color: Some(ConfigColor::rgb(8, 10, 14)),
        opacity: Some(44),
        blur_radius: Some(16),
    });
    config.visuals.weather = Some(WeatherVisualConfig {
        enabled: Some(true),
        size: Some(3),
        opacity: Some(62),
        icon_opacity: Some(41),
        temperature_opacity: Some(77),
        location_opacity: Some(53),
        temperature_color: Some(ConfigColor::rgb(255, 255, 255)),
        location_color: Some(ConfigColor::rgb(214, 227, 255)),
        temperature_font_family: Some(String::from("Prototype")),
        temperature_font_weight: Some(600),
        temperature_font_style: Some(FontStyle::Italic),
        temperature_letter_spacing: Some(2),
        location_font_family: Some(String::from("Geom")),
        location_font_weight: Some(500),
        location_font_style: Some(FontStyle::Italic),
        temperature_size: Some(4),
        location_size: Some(2),
        icon_size: Some(36),
        icon_gap: Some(10),
        location_gap: Some(3),
        alignment: Some(WeatherAlignment::Right),
        left_offset: Some(12),
        bottom_offset: Some(-6),
        left_padding: Some(56),
        horizontal_padding: Some(64),
        bottom_padding: Some(72),
    });
    config.visuals.now_playing = Some(NowPlayingVisualConfig {
        enabled: Some(true),
        fade_duration_ms: Some(320),
        opacity: Some(72),
        title_opacity: Some(88),
        artist_opacity: Some(54),
        artwork_opacity: Some(61),
        title_color: Some(ConfigColor::rgb(248, 251, 255)),
        artist_color: Some(ConfigColor::rgb(200, 212, 236)),
        title_font_family: Some("Geom".to_owned()),
        artist_font_family: Some("Prototype".to_owned()),
        title_font_weight: Some(700),
        artist_font_weight: Some(500),
        title_font_style: Some(FontStyle::Italic),
        artist_font_style: Some(FontStyle::Italic),
        title_size: Some(2),
        artist_size: Some(1),
        width: Some(280),
        content_gap: Some(18),
        text_gap: Some(10),
        artwork_size: Some(64),
        artwork_radius: Some(16),
        right_padding: Some(52),
        bottom_padding: Some(56),
        right_offset: Some(-6),
        bottom_offset: Some(10),
    });
    config.visuals.status = Some(StatusVisualConfig {
        enabled: Some(true),
        color: Some(ConfigColor::rgb(255, 224, 160)),
        opacity: Some(88),
        gap: Some(18),
    });
    config.visuals.layout = Some(LayoutVisualConfig {
        auth_stack_offset: Some(16),
        header_top_offset: Some(-12),
    });

    let theme = ShellTheme::from_config(&config);

    assert_eq!(theme.input.alpha, 26);
    assert_eq!(theme.input_border.alpha, 31);
    assert_eq!(
        theme.input_alignment,
        veila_common::InputAlignment::CenterCenter
    );
    assert_eq!(theme.input_horizontal_padding, Some(56));
    assert_eq!(theme.input_vertical_padding, Some(44));
    assert_eq!(theme.input_offset_x, Some(18));
    assert_eq!(theme.input_offset_y, Some(-12));
    assert_eq!(theme.input_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.input_font_weight, Some(600));
    assert_eq!(theme.input_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.input_font_size, Some(3));
    assert_eq!(theme.avatar_background, ClearColor::opaque(24, 30, 42));
    assert_eq!(theme.input_width, Some(280));
    assert_eq!(theme.input_height, Some(54));
    assert_eq!(theme.input_border_width, Some(3));
    assert_eq!(theme.avatar_size, Some(92));
    assert_eq!(theme.avatar_placeholder_padding, Some(14));
    assert_eq!(
        theme.avatar_icon_color,
        Some(ClearColor::opaque(232, 238, 249))
    );
    assert_eq!(
        theme.avatar_ring_color,
        Some(ClearColor::opaque(148, 178, 255))
    );
    assert_eq!(theme.avatar_ring_width, Some(3));
    assert_eq!(theme.avatar_background_opacity, Some(36));
    assert_eq!(
        theme.username_color,
        Some(ClearColor::opaque(215, 227, 255))
    );
    assert_eq!(theme.username_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.username_font_weight, Some(600));
    assert_eq!(theme.username_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.username_opacity, Some(72));
    assert_eq!(theme.username_size, Some(3));
    assert_eq!(theme.avatar_gap, Some(14));
    assert_eq!(theme.username_gap, Some(28));
    assert_eq!(theme.status_gap, Some(18));
    assert_eq!(theme.clock_gap, Some(10));
    assert_eq!(theme.auth_stack_offset, Some(16));
    assert_eq!(theme.header_top_offset, Some(-12));
    assert_eq!(theme.clock_font_family.as_deref(), Some("Bebas Neue"));
    assert_eq!(theme.clock_font_weight, Some(700));
    assert_eq!(theme.clock_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.clock_style, ClockStyle::Stacked);
    assert_eq!(theme.clock_alignment, ClockAlignment::TopLeft);
    assert_eq!(theme.clock_offset_x, Some(24));
    assert_eq!(theme.clock_offset_y, Some(-16));
    assert_eq!(theme.clock_format, ClockFormat::TwelveHour);
    assert_eq!(theme.clock_meridiem_size, Some(3));
    assert_eq!(theme.clock_meridiem_offset_x, Some(6));
    assert_eq!(theme.clock_meridiem_offset_y, Some(-2));
    assert_eq!(theme.clock_color, Some(ClearColor::opaque(248, 251, 255)));
    assert_eq!(theme.clock_opacity, Some(96));
    assert_eq!(theme.date_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.date_font_weight, Some(600));
    assert_eq!(theme.date_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.date_color, Some(ClearColor::opaque(200, 212, 236)));
    assert_eq!(theme.date_opacity, Some(74));
    assert_eq!(theme.clock_size, Some(4));
    assert_eq!(theme.date_size, Some(3));
    assert_eq!(
        theme.placeholder_color,
        Some(ClearColor::opaque(134, 148, 180))
    );
    assert_eq!(theme.placeholder_opacity, Some(60));
    assert_eq!(
        theme.eye_icon_color,
        Some(ClearColor::opaque(244, 248, 255))
    );
    assert_eq!(theme.eye_icon_opacity, Some(72));
    assert_eq!(
        theme.keyboard_background_color,
        ClearColor::rgba(18, 22, 30, 82)
    );
    assert_eq!(theme.keyboard_background_size, Some(42));
    assert_eq!(
        theme.keyboard_color,
        Some(ClearColor::opaque(232, 238, 249))
    );
    assert_eq!(theme.keyboard_opacity, Some(68));
    assert_eq!(theme.keyboard_size, Some(3));
    assert_eq!(theme.keyboard_top_offset, Some(-12));
    assert_eq!(theme.keyboard_right_offset, Some(8));
    assert_eq!(
        theme.battery_background_color,
        ClearColor::rgba(18, 22, 30, 82)
    );
    assert_eq!(theme.battery_color, Some(ClearColor::opaque(255, 255, 255)));
    assert_eq!(theme.battery_background_size, Some(42));
    assert_eq!(theme.battery_opacity, Some(72));
    assert_eq!(theme.battery_size, Some(18));
    assert_eq!(theme.battery_top_offset, Some(-12));
    assert_eq!(theme.battery_right_offset, Some(0));
    assert_eq!(theme.battery_gap, Some(8));
    assert!(theme.layer_enabled);
    assert_eq!(theme.layer_mode, LayerMode::Blur);
    assert_eq!(theme.layer_alignment, LayerAlignment::Right);
    assert_eq!(theme.layer_width, Some(520));
    assert_eq!(theme.layer_offset_x, Some(-12));
    assert_eq!(theme.layer_color, ClearColor::rgba(8, 10, 14, 112));
    assert_eq!(theme.layer_blur_radius, 16);
    assert_eq!(theme.weather_size, Some(3));
    assert_eq!(theme.weather_opacity, Some(62));
    assert_eq!(theme.weather_icon_opacity, Some(41));
    assert_eq!(theme.weather_temperature_opacity, Some(77));
    assert_eq!(theme.weather_location_opacity, Some(53));
    assert_eq!(
        theme.weather_temperature_color,
        Some(ClearColor::opaque(255, 255, 255))
    );
    assert_eq!(
        theme.weather_location_color,
        Some(ClearColor::opaque(214, 227, 255))
    );
    assert_eq!(
        theme.weather_temperature_font_family.as_deref(),
        Some("Prototype")
    );
    assert_eq!(theme.weather_temperature_font_weight, Some(600));
    assert_eq!(
        theme.weather_temperature_font_style,
        Some(FontStyle::Italic)
    );
    assert_eq!(theme.weather_location_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.weather_location_font_weight, Some(500));
    assert_eq!(theme.weather_location_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.weather_temperature_letter_spacing, Some(2));
    assert_eq!(theme.weather_temperature_size, Some(4));
    assert_eq!(theme.weather_location_size, Some(2));
    assert_eq!(theme.weather_icon_size, Some(36));
    assert_eq!(theme.weather_icon_gap, Some(10));
    assert_eq!(theme.weather_location_gap, Some(3));
    assert_eq!(theme.weather_alignment, WeatherAlignment::Right);
    assert_eq!(theme.weather_left_offset, Some(12));
    assert_eq!(theme.weather_bottom_offset, Some(-6));
    assert_eq!(theme.weather_horizontal_padding, Some(64));
    assert_eq!(theme.weather_bottom_padding, Some(72));
    assert_eq!(
        theme.now_playing_title_color,
        Some(ClearColor::opaque(248, 251, 255))
    );
    assert_eq!(
        theme.now_playing_artist_color,
        Some(ClearColor::opaque(200, 212, 236))
    );
    assert_eq!(theme.now_playing_fade_duration_ms, Some(320));
    assert_eq!(theme.now_playing_title_font_family.as_deref(), Some("Geom"));
    assert_eq!(
        theme.now_playing_artist_font_family.as_deref(),
        Some("Prototype")
    );
    assert_eq!(theme.now_playing_title_font_weight, Some(700));
    assert_eq!(theme.now_playing_artist_font_weight, Some(500));
    assert_eq!(theme.now_playing_title_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.now_playing_artist_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.now_playing_opacity, Some(72));
    assert_eq!(theme.now_playing_title_opacity, Some(88));
    assert_eq!(theme.now_playing_artist_opacity, Some(54));
    assert_eq!(theme.now_playing_artwork_opacity, Some(61));
    assert_eq!(theme.now_playing_title_size, Some(2));
    assert_eq!(theme.now_playing_artist_size, Some(1));
    assert_eq!(theme.now_playing_width, Some(280));
    assert_eq!(theme.now_playing_content_gap, Some(18));
    assert_eq!(theme.now_playing_text_gap, Some(10));
    assert_eq!(theme.now_playing_artwork_size, Some(64));
    assert_eq!(theme.now_playing_artwork_radius, Some(16));
    assert_eq!(theme.now_playing_right_padding, Some(52));
    assert_eq!(theme.now_playing_bottom_padding, Some(56));
    assert_eq!(theme.now_playing_right_offset, Some(-6));
    assert_eq!(theme.now_playing_bottom_offset, Some(10));
    assert_eq!(theme.status_color, Some(ClearColor::opaque(255, 224, 160)));
    assert_eq!(theme.status_opacity, Some(88));
    assert_eq!(
        theme.input_mask_color,
        Some(ClearColor::opaque(169, 196, 255))
    );
}

#[test]
fn nested_palette_overrides_flat_palette_keys() {
    let mut config = AppConfig::default();
    config.visuals.foreground = ConfigColor::rgb(10, 20, 30);
    config.visuals.palette = Some(PaletteVisualConfig {
        foreground: Some(ConfigColor::rgb(240, 244, 250)),
        muted: Some(ConfigColor::rgb(68, 78, 102)),
        pending: Some(ConfigColor::rgb(255, 194, 92)),
        rejected: Some(ConfigColor::rgb(220, 96, 96)),
    });

    let theme = ShellTheme::from_config(&config);

    assert_eq!(theme.foreground, ClearColor::opaque(240, 244, 250));
    assert_eq!(theme.muted, ClearColor::opaque(68, 78, 102));
    assert_eq!(theme.pending, ClearColor::opaque(255, 194, 92));
    assert_eq!(theme.rejected, ClearColor::opaque(220, 96, 96));
}

#[test]
fn avatar_background_falls_back_to_legacy_panel_color() {
    let mut config = AppConfig::default();
    config.visuals.panel = ConfigColor::rgb(31, 39, 52);
    config.visuals.avatar = Some(veila_common::AvatarVisualConfig {
        enabled: Some(true),
        background_color: None,
        ..Default::default()
    });
    config.visuals.avatar_background_color = None;

    let theme = ShellTheme::from_config(&config);

    assert_eq!(theme.avatar_background, ClearColor::opaque(31, 39, 52));
}
