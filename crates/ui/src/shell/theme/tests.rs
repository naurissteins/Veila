use veila_common::{
    AppConfig, AvatarVisualConfig, BackdropMode, BackdropShowWhen, BackdropVisualConfig,
    BatteryVisualConfig, ClockFormat, ClockStyle, ClockVisualConfig, ConfigColor, DateVisualConfig,
    EyeVisualConfig, FontStyle, GridVisualConfig, HorizontalAlign, InputRevealMode,
    InputVisualConfig, InputVisualEntry, KeyboardVisualConfig, LayerKind, LayerVisualConfig,
    NowPlayingArtworkVisualConfig, NowPlayingTextVisualConfig, NowPlayingVisualConfig,
    PaletteVisualConfig, PlaceholderVisualConfig, PowerStatusVisualConfig, RevealDisplayMode,
    RevealVisualConfig, StatusDisplayMode, StatusVisualConfig, UsernameVisualConfig, VerticalAlign,
    WeatherIconVisualConfig, WeatherLocationVisualConfig, WeatherTemperatureVisualConfig,
    WeatherVisualConfig, WidgetPositionConfig,
};
use veila_renderer::ClearColor;

use super::ShellTheme;

#[test]
fn input_alpha_uses_rgba_values() {
    let mut config = AppConfig::default();
    config.visuals.input = InputVisualEntry::Section(InputVisualConfig {
        placeholder: None,
        reveal_on_interaction: Some(true),
        reveal_mode: Some(InputRevealMode::Full),
        reveal_hint: None,
        font_family: Some(String::from("Geom")),
        font_weight: Some(600),
        font_style: Some(FontStyle::Italic),
        font_size: Some(22),
        background_color: Some(ConfigColor::rgba(255, 255, 255, 200)),
        border_color: Some(ConfigColor::rgba(255, 255, 255, 180)),
        width: Some(280),
        height: Some(54),
        radius: None,
        border_width: Some(3),
        mask_color: Some(ConfigColor::rgb(169, 196, 255)),
        position: WidgetPositionConfig::default(),
    });
    config.visuals.avatar = Some(AvatarVisualConfig {
        enabled: Some(true),
        image_path: None,
        size: Some(92),
        background_color: Some(ConfigColor::rgba(24, 30, 42, 92)),
        placeholder_padding: Some(14),
        ring_color: Some(ConfigColor::rgb(148, 178, 255)),
        ring_width: Some(3),
        icon_color: Some(ConfigColor::rgb(232, 238, 249)),
        position: WidgetPositionConfig::default(),
    });
    config.visuals.username = Some(UsernameVisualConfig {
        enabled: Some(true),
        text: None,
        font_family: Some(String::from("Geom")),
        font_weight: Some(600),
        font_style: Some(FontStyle::Italic),
        color: Some(ConfigColor::rgba(215, 227, 255, 184)),
        font_size: Some(22),
        position: WidgetPositionConfig::default(),
    });
    config.visuals.clock = Some(ClockVisualConfig {
        enabled: Some(true),
        font_family: Some(String::from("Bebas Neue")),
        font_weight: Some(700),
        font_style: Some(FontStyle::Italic),
        style: Some(ClockStyle::Stacked),
        format: Some(ClockFormat::TwelveHour),
        meridiem_font_size: Some(22),
        meridiem_x: Some(6),
        meridiem_y: Some(-2),
        color: Some(ConfigColor::rgba(248, 251, 255, 245)),
        font_size: Some(28),
        position: WidgetPositionConfig::default(),
    });
    config.visuals.date = Some(DateVisualConfig {
        enabled: Some(true),
        font_family: Some(String::from("Geom")),
        font_weight: Some(600),
        font_style: Some(FontStyle::Italic),
        color: Some(ConfigColor::rgba(200, 212, 236, 189)),
        font_size: Some(22),
        position: WidgetPositionConfig::default(),
    });
    config.visuals.placeholder = Some(PlaceholderVisualConfig {
        enabled: Some(true),
        color: Some(ConfigColor::rgba(134, 148, 180, 153)),
    });
    config.visuals.reveal = Some(RevealVisualConfig {
        mode: Some(RevealDisplayMode::Shown),
        text: Some(String::from("Press any key or click to unlock")),
        color: Some(ConfigColor::rgba(214, 227, 255, 168)),
        font_family: Some(String::from("Geom")),
        font_weight: Some(500),
        font_style: Some(FontStyle::Italic),
        font_size: Some(18),
    });
    config.visuals.eye = Some(EyeVisualConfig {
        enabled: Some(true),
        color: Some(ConfigColor::rgba(244, 248, 255, 184)),
    });
    config.visuals.keyboard = Some(KeyboardVisualConfig {
        enabled: Some(true),
        background_color: Some(ConfigColor::rgba(18, 22, 30, 82)),
        background_size: Some(42),
        color: Some(ConfigColor::rgba(232, 238, 249, 173)),
        size: Some(3),
        position: WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Top),
            x: Some(-24),
            y: Some(29),
            relative_to: None,
        },
    });
    config.visuals.battery = Some(BatteryVisualConfig {
        enabled: Some(true),
        background_color: Some(ConfigColor::rgba(18, 22, 30, 82)),
        background_size: Some(42),
        color: Some(ConfigColor::rgba(255, 255, 255, 184)),
        size: Some(18),
        position: WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Top),
            x: Some(-82),
            y: Some(29),
            relative_to: None,
        },
    });
    config.visuals.power_status = Some(PowerStatusVisualConfig {
        enabled: Some(true),
        position: WidgetPositionConfig {
            halign: Some(HorizontalAlign::Left),
            valign: Some(VerticalAlign::Bottom),
            x: Some(36),
            y: Some(-28),
            relative_to: None,
        },
    });
    config.visuals.grid = Some(GridVisualConfig {
        enabled: Some(true),
        cell_size: Some(48),
        color: Some(ConfigColor::rgba(255, 255, 255, 18)),
        major_every: Some(5),
        major_color: Some(ConfigColor::rgba(255, 255, 255, 44)),
    });
    config.visuals.backdrop = vec![BackdropVisualConfig {
        name: Some(String::from("panel")),
        enabled: Some(true),
        show_when: Some(BackdropShowWhen::NowPlaying),
        mode: Some(BackdropMode::Blur),
        color: Some(ConfigColor::rgba(8, 10, 14, 112)),
        blur_strength: Some(16),
        radius: Some(20),
        border_color: Some(ConfigColor::rgba(255, 255, 255, 48)),
        border_width: Some(2),
        full_width: Some(true),
        full_height: Some(false),
        inset_top: Some(14),
        inset_bottom: Some(18),
        inset_left: Some(22),
        inset_right: Some(26),
        width: Some(520),
        height: Some(420),
        z: Some(2),
        position: WidgetPositionConfig {
            halign: Some(HorizontalAlign::Right),
            valign: Some(VerticalAlign::Bottom),
            x: Some(-12),
            y: Some(16),
            relative_to: None,
        },
    }];
    config.visuals.layer = vec![LayerVisualConfig {
        name: Some(String::from("lock_icon")),
        enabled: Some(true),
        kind: Some(LayerKind::Icon),
        text: Some(String::from("\u{f023}")),
        font_family: Some(String::from("Symbols Nerd Font")),
        font_weight: Some(700),
        font_style: Some(FontStyle::Italic),
        font_size: Some(64),
        color: Some(ConfigColor::rgba(255, 255, 255, 238)),
        background_color: Some(ConfigColor::rgba(0, 0, 0, 64)),
        width: Some(120),
        height: Some(120),
        padding: Some(16),
        radius: Some(28),
        z: Some(20),
        position: WidgetPositionConfig {
            halign: Some(HorizontalAlign::Center),
            valign: Some(VerticalAlign::Center),
            x: Some(0),
            y: Some(-180),
            relative_to: Some(String::from("panel")),
        },
    }];
    config.visuals.weather = Some(WeatherVisualConfig {
        icon: Some(WeatherIconVisualConfig {
            enabled: Some(true),
            size: Some(36),
            opacity: Some(41),
            position: WidgetPositionConfig {
                halign: Some(HorizontalAlign::Right),
                valign: Some(VerticalAlign::Bottom),
                x: Some(-52),
                y: Some(-126),
                relative_to: None,
            },
        }),
        temperature: Some(WeatherTemperatureVisualConfig {
            enabled: Some(true),
            font_size: Some(40),
            font_family: Some(String::from("Prototype")),
            font_weight: Some(600),
            font_style: Some(FontStyle::Italic),
            letter_spacing: Some(2),
            color: Some(ConfigColor::rgba(255, 255, 255, 179)),
            position: WidgetPositionConfig {
                halign: Some(HorizontalAlign::Right),
                valign: Some(VerticalAlign::Bottom),
                x: Some(-52),
                y: Some(-80),
                relative_to: None,
            },
        }),
        location: Some(WeatherLocationVisualConfig {
            enabled: Some(true),
            font_size: Some(22),
            font_family: Some(String::from("Geom")),
            font_weight: Some(500),
            font_style: Some(FontStyle::Italic),
            color: Some(ConfigColor::rgba(214, 227, 255, 98)),
            position: WidgetPositionConfig {
                halign: Some(HorizontalAlign::Right),
                valign: Some(VerticalAlign::Bottom),
                x: Some(-52),
                y: Some(-52),
                relative_to: None,
            },
        }),
    });
    config.visuals.now_playing = Some(NowPlayingVisualConfig {
        enabled: Some(true),
        fade_duration_ms: Some(320),
        artwork: Some(NowPlayingArtworkVisualConfig {
            enabled: Some(true),
            size: Some(64),
            radius: Some(16),
            opacity: Some(61),
            position: WidgetPositionConfig {
                halign: Some(HorizontalAlign::Right),
                valign: Some(VerticalAlign::Bottom),
                x: Some(-274),
                y: Some(-46),
                relative_to: None,
            },
        }),
        artist: Some(NowPlayingTextVisualConfig {
            enabled: Some(true),
            width: Some(198),
            color: Some(ConfigColor::rgba(200, 212, 236, 99)),
            font_family: Some("Prototype".to_owned()),
            font_size: Some(10),
            font_weight: Some(500),
            font_style: Some(FontStyle::Italic),
            position: WidgetPositionConfig {
                halign: Some(HorizontalAlign::Right),
                valign: Some(VerticalAlign::Bottom),
                x: Some(-58),
                y: Some(-78),
                relative_to: None,
            },
        }),
        title: Some(NowPlayingTextVisualConfig {
            enabled: Some(true),
            width: Some(198),
            color: Some(ConfigColor::rgba(248, 251, 255, 208)),
            font_family: Some("Geom".to_owned()),
            font_size: Some(16),
            font_weight: Some(700),
            font_style: Some(FontStyle::Italic),
            position: WidgetPositionConfig {
                halign: Some(HorizontalAlign::Right),
                valign: Some(VerticalAlign::Bottom),
                x: Some(-58),
                y: Some(-46),
                relative_to: None,
            },
        }),
    });
    config.visuals.status = Some(StatusVisualConfig {
        enabled: Some(true),
        color: Some(ConfigColor::rgba(255, 224, 160, 224)),
        position: WidgetPositionConfig::default(),
        ..StatusVisualConfig::default()
    });
    let theme = ShellTheme::from_config(&config);

    assert_eq!(theme.input.alpha, 200);
    assert_eq!(theme.input_border.alpha, 180);
    assert!(theme.input_reveal_on_interaction);
    assert_eq!(theme.input_reveal_mode, InputRevealMode::Full);
    assert_eq!(theme.input_reveal_hint, "Press any key or click to unlock");
    assert!(theme.reveal_enabled);
    assert_eq!(
        theme.reveal_color,
        Some(ClearColor::rgba(214, 227, 255, 168))
    );
    assert_eq!(theme.reveal_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.reveal_font_weight, Some(500));
    assert_eq!(theme.reveal_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.reveal_font_size, Some(18));
    assert_eq!(theme.input_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.input_font_weight, Some(600));
    assert_eq!(theme.input_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.input_font_size, Some(22));
    assert_eq!(theme.avatar_background, ClearColor::rgba(24, 30, 42, 92));
    assert_eq!(theme.input_width, Some(280));
    assert_eq!(theme.input_height, Some(54));
    assert_eq!(theme.input_border_width, Some(3));
    assert_eq!(theme.avatar_size, Some(92));
    assert_eq!(theme.avatar_offset_y, Some(0));
    assert_eq!(theme.avatar_position, None);
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
    assert_eq!(
        theme.username_color,
        Some(ClearColor::rgba(215, 227, 255, 184))
    );
    assert_eq!(theme.username_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.username_font_weight, Some(600));
    assert_eq!(theme.username_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.username_font_size, Some(22));
    assert_eq!(theme.username_offset_y, Some(0));
    assert_eq!(theme.username_position, None);
    assert_eq!(theme.avatar_gap, Some(24));
    assert_eq!(theme.username_gap, Some(28));
    assert_eq!(theme.clock_gap, Some(20));
    assert_eq!(theme.clock_font_family.as_deref(), Some("Bebas Neue"));
    assert_eq!(theme.clock_font_weight, Some(700));
    assert_eq!(theme.clock_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.clock_style, ClockStyle::Stacked);
    assert_eq!(
        theme.clock_alignment,
        veila_common::ClockAlignment::TopCenter
    );
    assert!(!theme.clock_center_in_layer);
    assert_eq!(theme.clock_offset_x, Some(0));
    assert_eq!(theme.clock_offset_y, Some(0));
    assert_eq!(theme.clock_position, None);
    assert_eq!(theme.clock_format, ClockFormat::TwelveHour);
    assert_eq!(theme.clock_meridiem_font_size, Some(22));
    assert_eq!(theme.clock_meridiem_x, Some(6));
    assert_eq!(theme.clock_meridiem_y, Some(-2));
    assert_eq!(
        theme.clock_color,
        Some(ClearColor::rgba(248, 251, 255, 245))
    );
    assert_eq!(theme.date_font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.date_font_weight, Some(600));
    assert_eq!(theme.date_font_style, Some(FontStyle::Italic));
    assert_eq!(theme.date_color, Some(ClearColor::rgba(200, 212, 236, 189)));
    assert_eq!(theme.date_position, None);
    assert_eq!(theme.clock_font_size, Some(28));
    assert_eq!(theme.date_font_size, Some(22));
    assert_eq!(
        theme.placeholder_color,
        Some(ClearColor::rgba(134, 148, 180, 153))
    );
    assert_eq!(
        theme.eye_icon_color,
        Some(ClearColor::rgba(244, 248, 255, 184))
    );
    assert_eq!(
        theme.keyboard_background_color,
        ClearColor::rgba(18, 22, 30, 82)
    );
    assert_eq!(theme.keyboard_background_size, Some(42));
    assert_eq!(
        theme.keyboard_color,
        Some(ClearColor::rgba(232, 238, 249, 173))
    );
    assert_eq!(theme.keyboard_size, Some(3));
    assert_eq!(
        theme.keyboard_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Top,
            x: -24,
            y: 29,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.battery_background_color,
        ClearColor::rgba(18, 22, 30, 82)
    );
    assert_eq!(
        theme.battery_color,
        Some(ClearColor::rgba(255, 255, 255, 184))
    );
    assert_eq!(theme.battery_background_size, Some(42));
    assert_eq!(theme.battery_size, Some(18));
    assert_eq!(
        theme.battery_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Top,
            x: -82,
            y: 29,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert!(theme.power_status_enabled);
    assert_eq!(
        theme.power_status_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 36,
            y: -28,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(theme.backdrops.len(), 1);
    assert_eq!(theme.backdrops[0].mode, BackdropMode::Blur);
    assert_eq!(theme.backdrops[0].show_when, BackdropShowWhen::NowPlaying);
    assert_eq!(theme.backdrops[0].color, ClearColor::rgba(8, 10, 14, 112));
    assert_eq!(theme.backdrops[0].blur_strength, 16);
    assert_eq!(theme.backdrops[0].radius, 20);
    assert_eq!(
        theme.backdrops[0].border_color,
        Some(ClearColor::rgba(255, 255, 255, 48))
    );
    assert_eq!(theme.backdrops[0].border_width, 2);
    assert!(theme.backdrops[0].full_width);
    assert!(!theme.backdrops[0].full_height);
    assert_eq!(theme.backdrops[0].inset_top, 14);
    assert_eq!(theme.backdrops[0].inset_bottom, 18);
    assert_eq!(theme.backdrops[0].inset_left, 22);
    assert_eq!(theme.backdrops[0].inset_right, 26);
    assert_eq!(theme.backdrops[0].width, 520);
    assert_eq!(theme.backdrops[0].height, 420);
    assert_eq!(
        theme.backdrops[0].position,
        super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Bottom,
            x: -12,
            y: 16,
            target: super::WidgetPositionTarget::Screen,
        }
    );
    assert_eq!(theme.backdrops[0].z, 2);
    assert_eq!(theme.layers.len(), 1);
    assert_eq!(theme.layers[0].kind, LayerKind::Icon);
    assert_eq!(theme.layers[0].text, "\u{f023}");
    assert_eq!(
        theme.layers[0].font_family.as_deref(),
        Some("Symbols Nerd Font")
    );
    assert_eq!(theme.layers[0].font_weight, Some(700));
    assert_eq!(theme.layers[0].font_style, Some(FontStyle::Italic));
    assert_eq!(theme.layers[0].font_size, 64);
    assert_eq!(theme.layers[0].color, ClearColor::rgba(255, 255, 255, 238));
    assert_eq!(
        theme.layers[0].background_color,
        Some(ClearColor::rgba(0, 0, 0, 64))
    );
    assert_eq!(theme.layers[0].width, Some(120));
    assert_eq!(theme.layers[0].height, Some(120));
    assert_eq!(theme.layers[0].padding, 16);
    assert_eq!(theme.layers[0].radius, 28);
    assert_eq!(theme.layers[0].z, 20);
    assert_eq!(
        theme.layers[0].position,
        super::WidgetPosition {
            halign: HorizontalAlign::Center,
            valign: VerticalAlign::Center,
            x: 0,
            y: -180,
            target: super::WidgetPositionTarget::Backdrop(0),
        }
    );
    assert_eq!(
        theme.grid,
        Some(crate::shell::PreviewGrid {
            cell_size: 48,
            color: ClearColor::rgba(255, 255, 255, 18),
            major_every: 5,
            major_color: ClearColor::rgba(255, 255, 255, 44),
        })
    );
    assert!(theme.weather_icon_enabled);
    assert!(theme.weather_temperature_enabled);
    assert!(theme.weather_location_enabled);
    assert_eq!(theme.weather_icon_opacity, Some(41));
    assert_eq!(
        theme.weather_temperature_color,
        Some(ClearColor::rgba(255, 255, 255, 179))
    );
    assert_eq!(
        theme.weather_location_color,
        Some(ClearColor::rgba(214, 227, 255, 98))
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
    assert_eq!(theme.weather_temperature_font_size, Some(40));
    assert_eq!(theme.weather_location_font_size, Some(22));
    assert_eq!(theme.weather_icon_size, Some(36));
    assert_eq!(
        theme.weather_icon_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Bottom,
            x: -52,
            y: -126,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.weather_temperature_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Bottom,
            x: -52,
            y: -80,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.weather_location_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Bottom,
            x: -52,
            y: -52,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.now_playing_title_color,
        Some(ClearColor::rgba(248, 251, 255, 208))
    );
    assert_eq!(
        theme.now_playing_artist_color,
        Some(ClearColor::rgba(200, 212, 236, 99))
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
    assert!(theme.now_playing_artwork_enabled);
    assert!(theme.now_playing_artist_enabled);
    assert!(theme.now_playing_title_enabled);
    assert_eq!(theme.now_playing_artwork_opacity, Some(61));
    assert_eq!(theme.now_playing_title_font_size, Some(16));
    assert_eq!(theme.now_playing_artist_font_size, Some(10));
    assert_eq!(theme.now_playing_title_width, Some(198));
    assert_eq!(theme.now_playing_artist_width, Some(198));
    assert_eq!(theme.now_playing_artwork_size, Some(64));
    assert_eq!(theme.now_playing_artwork_radius, Some(16));
    assert_eq!(
        theme.now_playing_artwork_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Bottom,
            x: -274,
            y: -46,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.now_playing_artist_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Bottom,
            x: -58,
            y: -78,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.now_playing_title_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Bottom,
            x: -58,
            y: -46,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.status_color,
        Some(ClearColor::rgba(255, 224, 160, 224))
    );
    assert_eq!(theme.status_mode, StatusDisplayMode::Inline);
    assert_eq!(
        theme.input_mask_color,
        Some(ClearColor::opaque(169, 196, 255))
    );
}

#[test]
fn explicit_clock_and_date_positions_override_legacy_header_layout() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.clock]
            halign = "left"
            valign = "bottom"
            x = 20
            y = -40

            [visuals.date]
            halign = "right"
            valign = "top"
            x = -24
            y = 32
        "#,
    )
    .expect("position config should parse");

    let theme = ShellTheme::from_config(&config);

    assert_eq!(
        theme.clock_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 20,
            y: -40,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.date_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Top,
            x: -24,
            y: 32,
            target: super::WidgetPositionTarget::Screen,
        })
    );
}

#[test]
fn explicit_avatar_and_username_positions_override_legacy_auth_layout() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.avatar]
            halign = "right"
            valign = "top"
            x = -96
            y = 48

            [visuals.username]
            halign = "center"
            valign = "bottom"
            x = 0
            y = -72
        "#,
    )
    .expect("position config should parse");

    let theme = ShellTheme::from_config(&config);

    assert_eq!(
        theme.avatar_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Top,
            x: -96,
            y: 48,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.username_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Center,
            valign: VerticalAlign::Bottom,
            x: 0,
            y: -72,
            target: super::WidgetPositionTarget::Screen,
        })
    );
}

#[test]
fn explicit_input_and_status_positions_override_auth_flow_layout() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.input]
            halign = "left"
            valign = "bottom"
            x = 28
            y = -64

            [visuals.status]
            mode = "external"
            halign = "right"
            valign = "top"
            x = -32
            y = 48
        "#,
    )
    .expect("position config should parse");

    let theme = ShellTheme::from_config(&config);

    assert_eq!(
        theme.input_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 28,
            y: -64,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(
        theme.status_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Top,
            x: -32,
            y: 48,
            target: super::WidgetPositionTarget::Screen,
        })
    );
    assert_eq!(theme.status_mode, StatusDisplayMode::External);
}

#[test]
fn explicit_widget_position_can_target_named_backdrop() {
    let config = AppConfig::from_toml_str(
        r#"
            [[visuals.backdrop]]
            name = "auth_panel"
            width = 540
            full_height = true
            halign = "right"
            valign = "center"
            x = -100
            y = 0

            [visuals.clock]
            halign = "center"
            valign = "top"
            x = 0
            y = 40
            relative_to = "auth_panel"
        "#,
    )
    .expect("position config should parse");

    let theme = ShellTheme::from_config(&config);

    assert_eq!(
        theme.clock_position,
        Some(super::WidgetPosition {
            halign: HorizontalAlign::Center,
            valign: VerticalAlign::Top,
            x: 0,
            y: 40,
            target: super::WidgetPositionTarget::Backdrop(0),
        })
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

#[test]
fn render_scale_multiplies_theme_pixels_without_changing_colors() {
    let theme = ShellTheme {
        input_width: Some(310),
        input_height: Some(54),
        input_font_size: Some(18),
        clock_font_size: Some(88),
        input_position: Some(super::WidgetPosition {
            halign: HorizontalAlign::Center,
            valign: VerticalAlign::Bottom,
            x: 12,
            y: -24,
            target: super::WidgetPositionTarget::Screen,
        }),
        backdrops: vec![super::Backdrop {
            mode: BackdropMode::Blur,
            show_when: BackdropShowWhen::Always,
            color: ClearColor::rgba(20, 30, 40, 160),
            blur_strength: 18,
            radius: 24,
            border_color: Some(ClearColor::opaque(255, 255, 255)),
            border_width: 2,
            full_width: false,
            full_height: false,
            inset_top: 10,
            inset_bottom: 20,
            inset_left: 30,
            inset_right: 40,
            width: 400,
            height: 220,
            position: super::WidgetPosition {
                halign: HorizontalAlign::Right,
                valign: VerticalAlign::Center,
                x: -50,
                y: 8,
                target: super::WidgetPositionTarget::Screen,
            },
            z: 0,
        }],
        layers: vec![super::VisualLayer {
            kind: LayerKind::Text,
            text: String::from("locked"),
            color: ClearColor::opaque(255, 255, 255),
            background_color: Some(ClearColor::rgba(0, 0, 0, 64)),
            font_family: Some(String::from("Geom")),
            font_weight: Some(500),
            font_style: Some(FontStyle::Normal),
            font_size: 24,
            width: Some(120),
            height: Some(48),
            padding: 10,
            radius: 12,
            position: super::WidgetPosition {
                halign: HorizontalAlign::Center,
                valign: VerticalAlign::Center,
                x: 8,
                y: -16,
                target: super::WidgetPositionTarget::Screen,
            },
            z: 4,
        }],
        ..Default::default()
    };

    let scaled = theme.scaled_for_render(2);

    assert_eq!(scaled.input_width, Some(620));
    assert_eq!(scaled.input_height, Some(108));
    assert_eq!(scaled.input_font_size, Some(36));
    assert_eq!(scaled.clock_font_size, Some(176));
    assert_eq!(
        scaled
            .input_position
            .map(|position| (position.x, position.y)),
        Some((24, -48))
    );
    assert_eq!(scaled.backdrops[0].width, 800);
    assert_eq!(scaled.backdrops[0].height, 440);
    assert_eq!(scaled.backdrops[0].radius, 48);
    assert_eq!(scaled.backdrops[0].color, ClearColor::rgba(20, 30, 40, 160));
    assert_eq!(scaled.layers[0].font_size, 48);
    assert_eq!(scaled.layers[0].width, Some(240));
    assert_eq!(scaled.layers[0].height, Some(96));
    assert_eq!(scaled.layers[0].padding, 20);
    assert_eq!(scaled.layers[0].radius, 24);
    assert_eq!(
        (scaled.layers[0].position.x, scaled.layers[0].position.y),
        (16, -32)
    );
}
