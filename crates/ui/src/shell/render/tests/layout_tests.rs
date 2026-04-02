use super::*;

#[test]
fn backdrop_layer_rect_supports_center_and_right_alignment() {
    let centered = ShellState::new(
        ShellTheme {
            layer_enabled: true,
            layer_alignment: LayerAlignment::Center,
            layer_full_height: false,
            layer_width: Some(520),
            layer_height: Some(420),
            layer_vertical_alignment: LayerVerticalAlignment::Bottom,
            layer_top_padding: Some(18),
            layer_bottom_padding: Some(22),
            layer_offset_y: Some(-24),
            layer_mode: LayerMode::Blur,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let right = ShellState::new(
        ShellTheme {
            layer_enabled: true,
            layer_alignment: LayerAlignment::Right,
            layer_width: Some(520),
            layer_offset_x: Some(-12),
            layer_left_padding: Some(24),
            layer_right_padding: Some(36),
            layer_mode: LayerMode::Blur,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    let centered_rect = centered
        .backdrop_layer_rect(FrameSize::new(1280, 720))
        .expect("centered layer");
    let right_rect = right
        .backdrop_layer_rect(FrameSize::new(1280, 720))
        .expect("right layer");

    assert_eq!(centered_rect.x, 380);
    assert_eq!(centered_rect.y, 254);
    assert_eq!(centered_rect.width, 520);
    assert_eq!(centered_rect.height, 420);
    assert_eq!(right_rect.x, 712);
}

#[test]
fn bottom_center_auth_does_not_reserve_left_weather_footer_space() {
    let theme = ShellTheme {
        input_alignment: InputAlignment::BottomCenter,
        weather_enabled: true,
        weather_alignment: WeatherAlignment::Left,
        weather_horizontal_padding: Some(48),
        weather_bottom_padding: Some(48),
        ..ShellTheme::default()
    };
    let without_weather = ShellState::new(theme.clone(), None, None, true);
    let with_weather = ShellState::new_with_username_and_weather(
        theme,
        None,
        None,
        None,
        true,
        Some(String::from("Riga")),
        Some(WeatherSnapshot {
            temperature_celsius: 7,
            condition: WeatherCondition::Rain,
            fetched_at_unix: 0,
        }),
        WeatherUnit::Celsius,
        None,
    );

    let without_layout = without_weather.scene_layout(FrameSize::new(1280, 720));
    let with_layout = with_weather.scene_layout(FrameSize::new(1280, 720));

    assert_eq!(with_layout.anchors.auth_y, without_layout.anchors.auth_y);
}

#[test]
fn left_weather_footer_anchor_uses_real_widget_height() {
    let theme = ShellTheme {
        weather_enabled: true,
        weather_alignment: WeatherAlignment::Left,
        weather_horizontal_padding: Some(48),
        weather_bottom_padding: Some(48),
        ..ShellTheme::default()
    };
    let shell = ShellState::new_with_username_and_weather(
        theme,
        None,
        None,
        None,
        true,
        Some(String::from("Riga")),
        Some(WeatherSnapshot {
            temperature_celsius: 7,
            condition: WeatherCondition::Rain,
            fetched_at_unix: 0,
        }),
        WeatherUnit::Celsius,
        None,
    );

    let layout = shell.scene_layout(FrameSize::new(1280, 720));
    let footer_height =
        layout
            .model
            .total_height_for_role(LayoutRole::Footer, layout.metrics, &shell.status);

    assert_eq!(layout.anchors.footer_y, 720 - footer_height - 48);
}
