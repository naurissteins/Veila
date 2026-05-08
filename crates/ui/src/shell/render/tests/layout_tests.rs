use super::*;
use crate::shell::theme::{Backdrop, WidgetPosition};
use veila_common::{BackdropMode, WeatherUnit};

#[test]
fn backdrop_rect_supports_center_and_right_alignment() {
    let centered = ShellState::new(
        ShellTheme {
            backdrops: vec![Backdrop {
                mode: BackdropMode::Blur,
                color: ClearColor::rgba(8, 10, 14, 112),
                blur_strength: 16,
                radius: 20,
                border_color: Some(ClearColor::rgba(255, 255, 255, 48)),
                border_width: 2,
                width: 520,
                height: 420,
                position: WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Bottom,
                    x: 0,
                    y: -46,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let right = ShellState::new(
        ShellTheme {
            backdrops: vec![Backdrop {
                mode: BackdropMode::Blur,
                color: ClearColor::rgba(8, 10, 14, 112),
                blur_strength: 16,
                radius: 20,
                border_color: Some(ClearColor::rgba(255, 255, 255, 48)),
                border_width: 2,
                width: 520,
                height: 600,
                position: WidgetPosition {
                    halign: HorizontalAlign::Right,
                    valign: VerticalAlign::Top,
                    x: -12,
                    y: 0,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    let centered_rect =
        centered.backdrop_rect(FrameSize::new(1280, 720), centered.theme.backdrops[0]);
    let right_rect = right.backdrop_rect(FrameSize::new(1280, 720), right.theme.backdrops[0]);

    assert_eq!(centered_rect.x, 380);
    assert_eq!(centered_rect.y, 254);
    assert_eq!(centered_rect.width, 520);
    assert_eq!(centered_rect.height, 420);
    assert_eq!(right_rect.x, 748);
}

#[test]
fn preview_grid_renders_centered_major_and_minor_lines() {
    let mut shell = ShellState::new(
        ShellTheme {
            grid: Some(crate::shell::PreviewGrid {
                cell_size: 40,
                color: ClearColor::rgba(255, 255, 255, 20),
                major_every: 4,
                major_color: ClearColor::rgba(255, 255, 255, 38),
            }),
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    shell.set_preview_grid_enabled(true);

    let mut buffer = SoftwareBuffer::new(FrameSize::new(200, 120)).expect("buffer");
    buffer.clear(ClearColor::opaque(0, 0, 0));
    shell.render_overlay(&mut buffer);

    let center = &buffer.pixels()[(5 * 200 + 100) * 4..(5 * 200 + 100) * 4 + 4];
    let minor = &buffer.pixels()[(5 * 200 + 140) * 4..(5 * 200 + 140) * 4 + 4];
    let background = &buffer.pixels()[(19 * 200 + 119) * 4..(19 * 200 + 119) * 4 + 4];

    assert_eq!(center, &[38, 38, 38, 255]);
    assert_eq!(minor, &[20, 20, 20, 255]);
    assert_eq!(background, &[0, 0, 0, 255]);
}

#[test]
fn floating_weather_does_not_shift_auth_or_use_footer_role() {
    let theme = ShellTheme {
        weather_enabled: true,
        weather_icon_position: Some(crate::shell::theme::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 32,
            y: -120,
        }),
        weather_temperature_position: Some(crate::shell::theme::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 32,
            y: -72,
        }),
        weather_location_position: Some(crate::shell::theme::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 32,
            y: -40,
        }),
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
    assert!(with_layout.floating_weather.is_some());
    assert!(
        with_layout
            .model
            .sections_for_role(LayoutRole::Footer)
            .next()
            .is_none()
    );
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
