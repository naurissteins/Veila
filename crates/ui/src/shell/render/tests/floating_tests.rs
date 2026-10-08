use super::*;
use crate::shell::theme::{WidgetPosition, WidgetPositionTarget};
use veila_common::StatusDisplayMode;
use veila_renderer::{RenderScale, copy_rect_from};

fn position(valign: VerticalAlign) -> WidgetPosition {
    WidgetPosition {
        halign: HorizontalAlign::Left,
        valign,
        x: 24,
        y: if valign == VerticalAlign::Bottom {
            -64
        } else {
            32
        },
        target: WidgetPositionTarget::Screen,
    }
}

fn theme(valign: VerticalAlign) -> ShellTheme {
    ShellTheme {
        backdrops: Vec::new(),
        input_position: Some(position(valign)),
        status_mode: StatusDisplayMode::External,
        ..ShellTheme::default()
    }
}

fn rejected() -> ShellStatus {
    ShellStatus::Rejected {
        retry_until: None,
        displayed_retry_seconds: None,
        failed_attempts: Some(1),
        message: None,
    }
}

#[test]
fn floating_feedback_stays_outside_every_input_anchor() {
    for valign in [
        VerticalAlign::Top,
        VerticalAlign::Center,
        VerticalAlign::Bottom,
    ] {
        let mut shell = ShellState::new(theme(valign), None, None, true);
        shell.status = rejected();
        let layout = shell
            .render_context()
            .scene_layout(FrameSize::new(960, 640));
        let input = layout.floating_input.unwrap();
        let (status, _) = layout.floating_status.unwrap();
        assert_eq!(status.x, input.x + (input.width - status.width) / 2);
        if valign == VerticalAlign::Bottom {
            assert_eq!(status.y + status.height + 14, input.y);
        } else {
            assert_eq!(status.y, input.y + input.height + 14);
        }
    }
}

#[test]
fn explicit_feedback_position_overrides_input_anchor() {
    for valign in [
        VerticalAlign::Top,
        VerticalAlign::Center,
        VerticalAlign::Bottom,
    ] {
        let mut theme = theme(valign);
        theme.status_position = Some(WidgetPosition {
            halign: HorizontalAlign::Right,
            valign: VerticalAlign::Top,
            x: -32,
            y: 48,
            target: WidgetPositionTarget::Screen,
        });
        let mut shell = ShellState::new(theme, None, None, true);
        shell.status = rejected();
        let layout = shell
            .render_context()
            .scene_layout(FrameSize::new(960, 640));
        let (status, _) = layout.floating_status.unwrap();
        assert_eq!((status.x, status.y), (960 - status.width - 32, 48));
    }
}

#[test]
fn floating_auth_dirty_frame_matches_full_frame_at_every_scale() {
    for valign in [VerticalAlign::Top, VerticalAlign::Bottom] {
        for units in [120, 150, 180, 240] {
            let scale = RenderScale::from_units(units);
            let size = scale.frame_size(FrameSize::new(960, 640));
            let theme = ShellTheme {
                clock_enabled: false,
                date_enabled: false,
                avatar_enabled: false,
                username_enabled: false,
                ..theme(valign)
            };
            let mut shell = ShellState::new(theme, None, None, true);
            shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
            shell.handle_key(ShellKey::Character('x'));
            shell.status = ShellStatus::Notice {
                text: String::from("Notice"),
            };
            let mut base = SoftwareBuffer::solid(size, ClearColor::opaque(9, 12, 20)).unwrap();
            shell.render_static_overlay_at_scale(&mut base, scale);
            let mut actual = base.clone();
            shell.render_dynamic_overlay_at_scale(&mut actual, scale);
            let old_damage = shell.auth_dirty_rect_at_scale(size, scale).unwrap();
            let revision = shell.static_scene_revision();
            shell.status = rejected();
            assert_eq!(revision, shell.static_scene_revision());
            let damage = old_damage.union(shell.auth_dirty_rect_at_scale(size, scale).unwrap());
            copy_rect_from(&base, &mut actual, damage).unwrap();
            shell.render_auth_dirty_overlay_at_scale(&mut actual, scale);
            let mut expected = base;
            shell.render_dynamic_overlay_at_scale(&mut expected, scale);
            let mismatch = actual
                .pixels()
                .iter()
                .zip(expected.pixels())
                .position(|(a, b)| a != b);
            assert!(
                mismatch.is_none(),
                "anchor={valign:?}, scale={units}, first pixel={:?}, damage={damage:?}",
                mismatch.map(|i| ((i / 4) as u32 % size.width, (i / 4) as u32 / size.width))
            );
        }
    }
}

#[test]
fn floating_geometry_and_hitbox_refresh_after_theme_reload() {
    let mut shell = ShellState::new(theme(VerticalAlign::Top), None, None, true);
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    for units in [120, 150, 180, 240] {
        let scale = RenderScale::from_units(units);
        shell.with_pixel_scale(scale, |context| {
            context.scene_layout(scale.frame_size(FrameSize::new(960, 640)));
        });
    }
    let mut updated = theme(VerticalAlign::Bottom);
    updated.input_width = Some(280);
    updated.eye_enabled = true;
    updated.clock_position = Some(position(VerticalAlign::Top));
    updated.date_position = Some(position(VerticalAlign::Bottom));
    updated.avatar_position = Some(position(VerticalAlign::Top));
    updated.username_position = Some(position(VerticalAlign::Bottom));
    shell.apply_theme(updated.clone(), None, None, true);
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    shell.status = rejected();
    let mut fresh = ShellState::new(updated, None, None, true);
    fresh.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    fresh.status = rejected();
    for units in [120, 150, 180, 240] {
        let scale = RenderScale::from_units(units);
        let size = scale.frame_size(FrameSize::new(960, 640));
        let mut actual = SoftwareBuffer::solid(size, ClearColor::opaque(0, 0, 0)).unwrap();
        let mut expected = actual.clone();
        shell.render_at_scale(&mut actual, scale);
        fresh.render_at_scale(&mut expected, scale);
        assert_eq!(actual.pixels(), expected.pixels());
        assert_eq!(
            shell.widget_regions_at_scale(size, scale),
            fresh.widget_regions_at_scale(size, scale)
        );
        shell.with_pixel_scale(scale, |context| {
            fresh.with_pixel_scale(scale, |other| {
                let hitbox =
                    context.reveal_toggle_rect_for_frame(size.width as i32, size.height as i32);
                assert!(!hitbox.is_empty());
                assert_eq!(
                    hitbox,
                    other.reveal_toggle_rect_for_frame(size.width as i32, size.height as i32)
                );
            });
        });
    }
}
