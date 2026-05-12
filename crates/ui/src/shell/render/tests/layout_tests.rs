use super::*;
use crate::shell::theme::{Backdrop, VisualLayer, WidgetPosition, WidgetPositionTarget};
use veila_common::{
    BackdropMode, BackdropShowWhen, LayerKind, NowPlayingSnapshot, StatusDisplayMode, WeatherUnit,
};

#[test]
fn backdrop_rect_supports_center_and_right_alignment() {
    let centered = ShellState::new(
        ShellTheme {
            backdrops: vec![Backdrop {
                mode: BackdropMode::Blur,
                show_when: BackdropShowWhen::Always,
                color: ClearColor::rgba(8, 10, 14, 112),
                blur_strength: 16,
                radius: 20,
                border_color: Some(ClearColor::rgba(255, 255, 255, 48)),
                border_width: 2,
                full_width: false,
                full_height: false,
                inset_top: 0,
                inset_bottom: 0,
                inset_left: 0,
                inset_right: 0,
                width: 520,
                height: 420,
                position: WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Bottom,
                    x: 0,
                    y: -46,
                    target: WidgetPositionTarget::Screen,
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
                show_when: BackdropShowWhen::Always,
                color: ClearColor::rgba(8, 10, 14, 112),
                blur_strength: 16,
                radius: 20,
                border_color: Some(ClearColor::rgba(255, 255, 255, 48)),
                border_width: 2,
                full_width: false,
                full_height: false,
                inset_top: 0,
                inset_bottom: 0,
                inset_left: 0,
                inset_right: 0,
                width: 520,
                height: 600,
                position: WidgetPosition {
                    halign: HorizontalAlign::Right,
                    valign: VerticalAlign::Top,
                    x: -12,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    let centered_rect = centered.backdrop_rect(
        FrameSize::new(1280, 720),
        centered.theme.backdrops[0].clone(),
    );
    let right_rect =
        right.backdrop_rect(FrameSize::new(1280, 720), right.theme.backdrops[0].clone());

    assert_eq!(centered_rect.x, 380);
    assert_eq!(centered_rect.y, 254);
    assert_eq!(centered_rect.width, 520);
    assert_eq!(centered_rect.height, 420);
    assert_eq!(right_rect.x, 748);
}

#[test]
fn backdrop_rect_supports_full_width_and_height() {
    let shell = ShellState::new(
        ShellTheme {
            backdrops: vec![Backdrop {
                mode: BackdropMode::Blur,
                show_when: BackdropShowWhen::Always,
                color: ClearColor::rgba(8, 10, 14, 112),
                blur_strength: 16,
                radius: 20,
                border_color: Some(ClearColor::rgba(255, 255, 255, 48)),
                border_width: 2,
                full_width: true,
                full_height: true,
                inset_top: 0,
                inset_bottom: 0,
                inset_left: 0,
                inset_right: 0,
                width: 520,
                height: 420,
                position: WidgetPosition {
                    halign: HorizontalAlign::Right,
                    valign: VerticalAlign::Bottom,
                    x: -12,
                    y: -16,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    let rect = shell.backdrop_rect(FrameSize::new(1280, 720), shell.theme.backdrops[0].clone());

    assert_eq!(rect.x, 0);
    assert_eq!(rect.y, 0);
    assert_eq!(rect.width, 1280);
    assert_eq!(rect.height, 720);
}

#[test]
fn backdrop_rect_applies_full_height_insets() {
    let shell = ShellState::new(
        ShellTheme {
            backdrops: vec![Backdrop {
                mode: BackdropMode::Blur,
                show_when: BackdropShowWhen::Always,
                color: ClearColor::rgba(8, 10, 14, 112),
                blur_strength: 16,
                radius: 20,
                border_color: Some(ClearColor::rgba(255, 255, 255, 48)),
                border_width: 2,
                full_width: false,
                full_height: true,
                inset_top: 110,
                inset_bottom: 110,
                inset_left: 0,
                inset_right: 0,
                width: 540,
                height: 420,
                position: WidgetPosition {
                    halign: HorizontalAlign::Left,
                    valign: VerticalAlign::Center,
                    x: 110,
                    y: 999,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    let rect = shell.backdrop_rect(FrameSize::new(1280, 720), shell.theme.backdrops[0].clone());

    assert_eq!(rect.x, 110);
    assert_eq!(rect.y, 110);
    assert_eq!(rect.width, 540);
    assert_eq!(rect.height, 500);
}

#[test]
fn widget_position_can_center_inside_backdrop_rect() {
    let shell = ShellState::new(
        ShellTheme {
            backdrops: vec![Backdrop {
                mode: BackdropMode::Blur,
                show_when: BackdropShowWhen::Always,
                color: ClearColor::rgba(8, 10, 14, 112),
                blur_strength: 16,
                radius: 20,
                border_color: Some(ClearColor::rgba(255, 255, 255, 48)),
                border_width: 2,
                full_width: false,
                full_height: true,
                inset_top: 0,
                inset_bottom: 0,
                inset_left: 0,
                inset_right: 0,
                width: 540,
                height: 420,
                position: WidgetPosition {
                    halign: HorizontalAlign::Right,
                    valign: VerticalAlign::Center,
                    x: -100,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    let rect = shell.positioned_rect(
        FrameSize::new(1280, 720),
        WidgetPosition {
            halign: HorizontalAlign::Center,
            valign: VerticalAlign::Top,
            x: 0,
            y: 40,
            target: WidgetPositionTarget::Backdrop(0),
        },
        300,
        120,
    );

    assert_eq!(rect.x, 760);
    assert_eq!(rect.y, 40);
    assert_eq!(rect.width, 300);
    assert_eq!(rect.height, 120);
}

#[test]
fn conditional_now_playing_backdrop_renders_only_when_widget_is_visible() {
    let theme = ShellTheme {
        now_playing_enabled: true,
        backdrops: vec![Backdrop {
            mode: BackdropMode::Solid,
            show_when: BackdropShowWhen::NowPlaying,
            color: ClearColor::opaque(255, 0, 0),
            blur_strength: 0,
            radius: 0,
            border_color: None,
            border_width: 0,
            full_width: false,
            full_height: false,
            inset_top: 0,
            inset_bottom: 0,
            inset_left: 0,
            inset_right: 0,
            width: 120,
            height: 80,
            position: WidgetPosition {
                halign: HorizontalAlign::Center,
                valign: VerticalAlign::Center,
                x: 0,
                y: 0,
                target: WidgetPositionTarget::Screen,
            },
            z: 0,
        }],
        ..ShellTheme::default()
    };

    let hidden = ShellState::new(theme.clone(), None, None, true);
    let visible = ShellState::new_with_username_and_widgets(
        theme,
        None,
        None,
        None,
        true,
        None,
        None,
        WeatherUnit::default(),
        None,
        Some(NowPlayingSnapshot {
            title: String::from("Track"),
            artist: Some(String::from("Artist")),
            artwork_path: None,
            fetched_at_unix: 0,
        }),
    );

    let mut hidden_buffer = SoftwareBuffer::new(FrameSize::new(200, 120)).expect("buffer");
    hidden_buffer.clear(ClearColor::opaque(0, 0, 0));
    hidden.render_backdrops(&mut hidden_buffer);

    let mut visible_buffer = SoftwareBuffer::new(FrameSize::new(200, 120)).expect("buffer");
    visible_buffer.clear(ClearColor::opaque(0, 0, 0));
    visible.render_backdrops(&mut visible_buffer);

    let hidden_center = &hidden_buffer.pixels()[(60 * 200 + 100) * 4..(60 * 200 + 100) * 4 + 4];
    let visible_center = &visible_buffer.pixels()[(60 * 200 + 100) * 4..(60 * 200 + 100) * 4 + 4];

    assert_eq!(hidden_center, &[0, 0, 0, 255]);
    assert_eq!(visible_center, &[0, 0, 255, 255]);
}

#[test]
fn custom_visual_layer_renders_background_surface() {
    let shell = ShellState::new(
        ShellTheme {
            background: ClearColor::rgba(0, 0, 0, 0),
            layers: vec![VisualLayer {
                kind: LayerKind::Text,
                text: String::from("."),
                color: ClearColor::opaque(255, 255, 255),
                background_color: Some(ClearColor::opaque(10, 20, 30)),
                font_family: None,
                font_weight: None,
                font_style: None,
                font_size: 1,
                width: Some(40),
                height: Some(20),
                padding: 0,
                radius: 0,
                position: WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Center,
                    x: 0,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 80)).expect("buffer");

    shell.render_layers(&mut buffer);

    let inside = &buffer.pixels()[(31 * 120 + 41) * 4..(31 * 120 + 41) * 4 + 4];
    let outside = &buffer.pixels()[(20 * 120 + 20) * 4..(20 * 120 + 20) * 4 + 4];
    assert_eq!(inside, &[30, 20, 10, 255]);
    assert_eq!(outside, &[0, 0, 0, 0]);
}

#[test]
fn static_overlay_includes_custom_visual_layers() {
    let shell = ShellState::new(
        ShellTheme {
            background: ClearColor::rgba(0, 0, 0, 0),
            avatar_enabled: false,
            username_enabled: false,
            clock_enabled: false,
            date_enabled: false,
            layers: vec![VisualLayer {
                kind: LayerKind::Text,
                text: String::from("."),
                color: ClearColor::opaque(255, 255, 255),
                background_color: Some(ClearColor::opaque(12, 34, 56)),
                font_family: None,
                font_weight: None,
                font_style: None,
                font_size: 1,
                width: Some(40),
                height: Some(20),
                padding: 0,
                radius: 0,
                position: WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Center,
                    x: 0,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 80)).expect("buffer");

    shell.render_static_overlay(&mut buffer);

    let inside = &buffer.pixels()[(31 * 120 + 41) * 4..(31 * 120 + 41) * 4 + 4];
    assert_eq!(inside, &[56, 34, 12, 255]);
}

#[test]
fn static_overlay_without_layers_omits_custom_visual_layers() {
    let shell = ShellState::new(
        ShellTheme {
            background: ClearColor::rgba(0, 0, 0, 0),
            avatar_enabled: false,
            username_enabled: false,
            clock_enabled: false,
            date_enabled: false,
            layers: vec![VisualLayer {
                kind: LayerKind::Text,
                text: String::from("."),
                color: ClearColor::opaque(255, 255, 255),
                background_color: Some(ClearColor::opaque(12, 34, 56)),
                font_family: None,
                font_weight: None,
                font_style: None,
                font_size: 1,
                width: Some(40),
                height: Some(20),
                padding: 0,
                radius: 0,
                position: WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Center,
                    x: 0,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 80)).expect("buffer");

    shell.render_static_overlay_without_layers(&mut buffer);

    let inside = &buffer.pixels()[(31 * 120 + 41) * 4..(31 * 120 + 41) * 4 + 4];
    assert_eq!(inside, &[0, 0, 0, 0]);
}

#[test]
fn icon_visual_layer_centers_visible_glyph_bounds() {
    let shell = ShellState::new(
        ShellTheme {
            background: ClearColor::rgba(0, 0, 0, 0),
            layers: vec![VisualLayer {
                kind: LayerKind::Icon,
                text: String::from("i"),
                color: ClearColor::opaque(255, 255, 255),
                background_color: None,
                font_family: None,
                font_weight: None,
                font_style: None,
                font_size: 48,
                width: Some(120),
                height: Some(120),
                padding: 0,
                radius: 0,
                position: WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Center,
                    x: 0,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 120)).expect("buffer");

    shell.render_layers(&mut buffer);

    let (left, right) = visible_alpha_x_bounds(&buffer).expect("visible glyph bounds");
    let center = (left + right) / 2;
    assert!((center - 60).abs() <= 1, "glyph center was {center}");
}

#[test]
fn preview_grid_renders_centered_major_and_minor_lines() {
    let mut shell = ShellState::new(
        ShellTheme {
            background: ClearColor::opaque(0, 0, 0),
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

    assert_eq!(center, &[47, 47, 47, 255]);
    assert_eq!(minor, &[20, 20, 20, 255]);
    assert_eq!(background, &[0, 0, 0, 255]);
}

fn visible_alpha_x_bounds(buffer: &SoftwareBuffer) -> Option<(i32, i32)> {
    let width = buffer.size().width as usize;
    let mut left = width;
    let mut right = 0usize;

    for (index, pixel) in buffer.pixels().chunks_exact(4).enumerate() {
        if pixel[3] == 0 {
            continue;
        }

        let x = index % width;
        left = left.min(x);
        right = right.max(x + 1);
    }

    (left < right).then_some((left as i32, right as i32))
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
            target: WidgetPositionTarget::Screen,
        }),
        weather_temperature_position: Some(crate::shell::theme::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 32,
            y: -72,
            target: WidgetPositionTarget::Screen,
        }),
        weather_location_position: Some(crate::shell::theme::WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Bottom,
            x: 32,
            y: -40,
            target: WidgetPositionTarget::Screen,
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
                target: WidgetPositionTarget::Screen,
            }),
            username_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Left,
                valign: VerticalAlign::Top,
                x: 24,
                y: 200,
                target: WidgetPositionTarget::Screen,
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
                target: WidgetPositionTarget::Screen,
            }),
            username_position: None,
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
                target: WidgetPositionTarget::Screen,
            }),
            status_mode: StatusDisplayMode::External,
            status_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Right,
                valign: VerticalAlign::Top,
                x: -32,
                y: 48,
                target: WidgetPositionTarget::Screen,
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
fn inline_status_stays_inside_explicit_input_by_default() {
    let shell = ShellState::new_with_username(
        ShellTheme {
            input_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Left,
                valign: VerticalAlign::Bottom,
                x: 24,
                y: -64,
                target: WidgetPositionTarget::Screen,
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
    assert!(layout.floating_status.is_none());
    assert!(!layout.floating_status_follows_input);
}

#[test]
fn external_status_follows_explicit_input_when_status_position_is_unset() {
    let shell = ShellState::new_with_username(
        ShellTheme {
            input_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Left,
                valign: VerticalAlign::Bottom,
                x: 24,
                y: -64,
                target: WidgetPositionTarget::Screen,
            }),
            status_mode: StatusDisplayMode::External,
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

#[test]
fn hidden_status_mode_removes_auth_feedback_from_layout() {
    let mut shell = ShellState::new_with_username(
        ShellTheme {
            status_mode: StatusDisplayMode::Hidden,
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

    assert!(layout.floating_status.is_none());
    assert!(
        layout
            .model
            .sections_for_role(LayoutRole::Auth)
            .all(|section| !matches!(section.widget, SceneWidget::Status(_)))
    );
}
