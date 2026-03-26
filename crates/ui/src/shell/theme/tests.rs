use veila_common::{
    AppConfig, AvatarVisualConfig, ClockVisualConfig, ConfigColor, DateVisualConfig,
    EyeVisualConfig, InputVisualConfig, InputVisualEntry, LayoutVisualConfig, PaletteVisualConfig,
    PlaceholderVisualConfig, StatusVisualConfig, UsernameVisualConfig, WeatherAlignment,
    WeatherVisualConfig,
};
use veila_renderer::ClearColor;

use super::ShellTheme;

#[test]
fn input_opacity_overrides_embedded_alpha() {
    let mut config = AppConfig::default();
    config.visuals.input = InputVisualEntry::Section(InputVisualConfig {
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
        color: Some(ConfigColor::rgb(215, 227, 255)),
        opacity: Some(72),
        size: Some(3),
        gap: Some(28),
    });
    config.visuals.clock = Some(ClockVisualConfig {
        font_family: Some(String::from("Bebas Neue")),
        color: Some(ConfigColor::rgb(248, 251, 255)),
        opacity: Some(96),
        size: Some(4),
        gap: Some(10),
    });
    config.visuals.date = Some(DateVisualConfig {
        color: Some(ConfigColor::rgb(200, 212, 236)),
        opacity: Some(74),
        size: Some(3),
    });
    config.visuals.placeholder = Some(PlaceholderVisualConfig {
        color: Some(ConfigColor::rgb(134, 148, 180)),
        opacity: Some(60),
    });
    config.visuals.eye = Some(EyeVisualConfig {
        color: Some(ConfigColor::rgb(244, 248, 255)),
        opacity: Some(72),
    });
    config.visuals.weather = Some(WeatherVisualConfig {
        size: Some(3),
        opacity: Some(62),
        icon_opacity: Some(41),
        temperature_opacity: Some(77),
        location_opacity: Some(53),
        temperature_color: Some(ConfigColor::rgb(255, 255, 255)),
        location_color: Some(ConfigColor::rgb(214, 227, 255)),
        temperature_font_family: Some(String::from("Prototype")),
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
    config.visuals.status = Some(StatusVisualConfig {
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
    assert_eq!(theme.username_opacity, Some(72));
    assert_eq!(theme.username_size, Some(3));
    assert_eq!(theme.avatar_gap, Some(14));
    assert_eq!(theme.username_gap, Some(28));
    assert_eq!(theme.status_gap, Some(18));
    assert_eq!(theme.clock_gap, Some(10));
    assert_eq!(theme.auth_stack_offset, Some(16));
    assert_eq!(theme.header_top_offset, Some(-12));
    assert_eq!(theme.clock_font_family.as_deref(), Some("Bebas Neue"));
    assert_eq!(theme.clock_color, Some(ClearColor::opaque(248, 251, 255)));
    assert_eq!(theme.clock_opacity, Some(96));
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

    let theme = ShellTheme::from_config(&config);

    assert_eq!(theme.avatar_background, ClearColor::opaque(31, 39, 52));
}
