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

#[test]
fn explicit_avatar_and_username_positions_are_removed_from_auth_flow() {
    let shell = ShellState::new_with_username(
        ShellTheme {
            avatar_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Left,
                valign: VerticalAlign::Top,
                x: 24,
                y: 32,
            }),
            username_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Left,
                valign: VerticalAlign::Top,
                x: 24,
                y: 200,
            }),
            ..ShellTheme::default()
        },
        None,
        Some(String::from("ns")),
        None,
        true,
    );

    let layout = shell.scene_layout(FrameSize::new(1280, 720));

    assert!(layout.floating_avatar);
    assert!(layout.floating_username.is_some());
    assert!(
        layout
            .model
            .sections_for_role(LayoutRole::Auth)
            .all(|section| !matches!(
                section.widget,
                SceneWidget::Avatar | SceneWidget::Username(_)
            ))
    );
}

#[test]
fn username_stays_in_auth_flow_when_only_avatar_is_explicit() {
    let shell = ShellState::new_with_username(
        ShellTheme {
            avatar_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Center,
                valign: VerticalAlign::Center,
                x: 12,
                y: -48,
            }),
            ..ShellTheme::default()
        },
        None,
        Some(String::from("ns")),
        None,
        true,
    );

    let layout = shell.scene_layout(FrameSize::new(1280, 720));

    assert!(layout.floating_avatar);
    assert!(layout.floating_username.is_none());
    assert!(
        layout
            .model
            .sections_for_role(LayoutRole::Auth)
            .any(|section| matches!(section.widget, SceneWidget::Username(_)))
    );
}

#[test]
fn explicit_input_and_status_positions_are_removed_from_auth_flow() {
    let mut shell = ShellState::new_with_username(
        ShellTheme {
            input_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Center,
                valign: VerticalAlign::Bottom,
                x: 0,
                y: -72,
            }),
            status_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Right,
                valign: VerticalAlign::Top,
                x: -32,
                y: 48,
            }),
            ..ShellTheme::default()
        },
        None,
        Some(String::from("ns")),
        None,
        true,
    );
    shell.status = ShellStatus::Rejected {
        retry_until: None,
        displayed_retry_seconds: None,
        failed_attempts: Some(1),
    };

    let layout = shell.scene_layout(FrameSize::new(1280, 720));

    assert!(layout.floating_input);
    assert!(layout.floating_status.is_some());
    assert!(
        layout
            .model
            .sections_for_role(LayoutRole::Auth)
            .all(|section| !matches!(
                section.widget,
                SceneWidget::Input(_) | SceneWidget::Status(_)
            ))
    );
}

#[test]
fn status_follows_explicit_input_when_status_position_is_unset() {
    let shell = ShellState::new_with_username(
        ShellTheme {
            input_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Left,
                valign: VerticalAlign::Bottom,
                x: 24,
                y: -64,
            }),
            ..ShellTheme::default()
        },
        None,
        Some(String::from("ns")),
        None,
        true,
    );
    let mut shell = shell;
    shell.status = ShellStatus::Rejected {
        retry_until: None,
        displayed_retry_seconds: None,
        failed_attempts: Some(1),
    };

    let layout = shell.scene_layout(FrameSize::new(1280, 720));

    assert!(layout.floating_input);
    assert!(layout.floating_status.is_some());
    assert!(layout.floating_status_follows_input);
    assert!(
        layout
            .model
            .sections_for_role(LayoutRole::Auth)
            .all(|section| !matches!(section.widget, SceneWidget::Status(_)))
    );
}
