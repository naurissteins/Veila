use veila_common::{AppConfig, ConfigColor};
use veila_renderer::ClearColor;

use super::ShellTheme;

#[test]
fn input_opacity_overrides_embedded_alpha() {
    let mut config = AppConfig::default();
    config.visuals.input = ConfigColor::rgba(255, 255, 255, 200);
    config.visuals.input_opacity = Some(10);
    config.visuals.input_border = ConfigColor::rgba(255, 255, 255, 180);
    config.visuals.input_border_opacity = Some(12);
    config.visuals.avatar_background_color = Some(ConfigColor::rgb(24, 30, 42));
    config.visuals.input_width = Some(280);
    config.visuals.input_height = Some(54);
    config.visuals.avatar_size = Some(92);
    config.visuals.input_border_width = Some(3);
    config.visuals.avatar_placeholder_padding = Some(14);
    config.visuals.avatar_icon_color = Some(ConfigColor::rgb(232, 238, 249));
    config.visuals.avatar_ring_color = Some(ConfigColor::rgb(148, 178, 255));
    config.visuals.avatar_ring_width = Some(3);
    config.visuals.avatar_background_opacity = Some(36);
    config.visuals.username_color = Some(ConfigColor::rgb(215, 227, 255));
    config.visuals.username_opacity = Some(72);
    config.visuals.username_size = Some(3);
    config.visuals.avatar_gap = Some(14);
    config.visuals.username_gap = Some(28);
    config.visuals.status_gap = Some(18);
    config.visuals.clock_gap = Some(10);
    config.visuals.auth_stack_offset = Some(16);
    config.visuals.header_top_offset = Some(-12);
    config.visuals.clock_font_family = Some(String::from("Bebas Neue"));
    config.visuals.clock_color = Some(ConfigColor::rgb(248, 251, 255));
    config.visuals.clock_opacity = Some(96);
    config.visuals.date_color = Some(ConfigColor::rgb(200, 212, 236));
    config.visuals.date_opacity = Some(74);
    config.visuals.clock_size = Some(4);
    config.visuals.date_size = Some(3);
    config.visuals.placeholder_color = Some(ConfigColor::rgb(134, 148, 180));
    config.visuals.placeholder_opacity = Some(60);
    config.visuals.eye_icon_color = Some(ConfigColor::rgb(244, 248, 255));
    config.visuals.eye_icon_opacity = Some(72);
    config.visuals.status_color = Some(ConfigColor::rgb(255, 224, 160));
    config.visuals.status_opacity = Some(88);
    config.visuals.input_mask_color = Some(ConfigColor::rgb(169, 196, 255));

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
    assert_eq!(theme.status_color, Some(ClearColor::opaque(255, 224, 160)));
    assert_eq!(theme.status_opacity, Some(88));
    assert_eq!(
        theme.input_mask_color,
        Some(ClearColor::opaque(169, 196, 255))
    );
}

#[test]
fn avatar_background_falls_back_to_legacy_panel_color() {
    let mut config = AppConfig::default();
    config.visuals.panel = ConfigColor::rgb(31, 39, 52);

    let theme = ShellTheme::from_config(&config);

    assert_eq!(theme.avatar_background, ClearColor::opaque(31, 39, 52));
}
