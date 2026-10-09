use super::*;
use crate::shell::theme::{Backdrop, WidgetPosition, WidgetPositionTarget};
use crate::shell::{ShellState, ShellTheme};
use crate::{ClockTheme, DateTheme};
use veila_common::{BackdropMode, HorizontalAlign, NowPlayingSnapshot, VerticalAlign};
use veila_renderer::{ClearColor, RenderScale, SoftwareBuffer, copy_rect_from};

fn position(halign: HorizontalAlign, valign: VerticalAlign, x: i32, y: i32) -> WidgetPosition {
    WidgetPosition {
        halign,
        valign,
        x,
        y,
        target: WidgetPositionTarget::Screen,
    }
}

fn verify_widget_change(
    shell: &mut ShellState,
    scale: RenderScale,
    widget: WidgetKind,
    change: impl FnOnce(&mut ShellState),
) {
    let size = scale.frame_size(FrameSize::new(800, 600));
    let mut base = SoftwareBuffer::solid(size, ClearColor::opaque(9, 12, 20)).unwrap();
    shell.render_static_overlay_at_scale(&mut base, scale);
    let old_regions = shell.widget_regions_at_scale(size, scale);
    let mut actual = base.clone();
    shell.render_dynamic_overlay_at_scale(&mut actual, scale);

    change(shell);
    let current = shell.widget_regions_at_scale(size, scale);
    let WidgetDamage::Region(damage) = current.damage_since(old_regions, widget) else {
        panic!("widget should use a dirty region");
    };
    assert!(
        u64::from(damage.width as u32) * u64::from(damage.height as u32)
            < u64::from(size.width) * u64::from(size.height) / 2
    );
    copy_rect_from(&base, &mut actual, damage).unwrap();
    shell.render_widget_at_scale(&mut actual, scale, widget);

    let mut expected = base;
    shell.render_dynamic_overlay_at_scale(&mut expected, scale);
    assert_eq!(actual.pixels(), expected.pixels());
}

#[test]
fn clock_minute_partial_frame_matches_full_render_at_fractional_scale() {
    let mut shell = ShellState::new(
        ShellTheme {
            clock: ClockTheme {
                enabled: true,
                ..ShellTheme::default().clock
            },
            date: DateTheme {
                enabled: true,
                ..ShellTheme::default().date
            },
            backdrops: Vec::new(),
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    verify_widget_change(
        &mut shell,
        RenderScale::from_units(180),
        WidgetKind::Header,
        |shell| {
            shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(1));
        },
    );
}

#[test]
fn keyboard_chip_partial_frame_matches_full_render() {
    let mut shell = ShellState::new(
        ShellTheme {
            backdrops: Vec::new(),
            keyboard_enabled: true,
            keyboard_position: Some(position(
                HorizontalAlign::Right,
                VerticalAlign::Top,
                -12,
                12,
            )),
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    shell.set_keyboard_layout_label(Some(String::from("EN")));
    verify_widget_change(
        &mut shell,
        RenderScale::ONE,
        WidgetKind::Indicators,
        |shell| {
            shell.set_keyboard_layout_label(Some(String::from("Finnish")));
        },
    );
}

#[test]
fn media_change_partial_frame_matches_full_render() {
    let mut shell = ShellState::new(
        ShellTheme {
            backdrops: Vec::new(),
            now_playing_enabled: true,
            now_playing_title_enabled: true,
            now_playing_title_position: Some(position(
                HorizontalAlign::Left,
                VerticalAlign::Bottom,
                24,
                -24,
            )),
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
        title: String::from("Old title"),
        artist: None,
        artwork_path: None,
        fetched_at_unix: 0,
    }));
    shell.now_playing_transition = None;
    verify_widget_change(
        &mut shell,
        RenderScale::from_units(150),
        WidgetKind::Media,
        |shell| {
            shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
                title: String::from("A longer track title"),
                artist: None,
                artwork_path: None,
                fetched_at_unix: 1,
            }));
            shell.now_playing_transition = None;
        },
    );
}

#[test]
fn media_backdrop_appearance_and_removal_match_full_render() {
    let mut shell = ShellState::new(
        ShellTheme {
            backdrops: vec![Backdrop {
                mode: BackdropMode::Blur,
                show_when: BackdropShowWhen::NowPlaying,
                color: ClearColor::opaque(34, 52, 76),
                blur_strength: 12,
                radius: 8,
                border_color: None,
                border_width: 0,
                full_width: false,
                full_height: false,
                inset_top: 0,
                inset_bottom: 0,
                inset_left: 0,
                inset_right: 0,
                width: 320,
                height: 88,
                rotate: 0,
                position: position(HorizontalAlign::Left, VerticalAlign::Bottom, 16, -16),
                z: 0,
            }],
            now_playing_enabled: true,
            now_playing_title_enabled: true,
            now_playing_title_position: Some(position(
                HorizontalAlign::Left,
                VerticalAlign::Bottom,
                24,
                -24,
            )),
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    verify_widget_change(&mut shell, RenderScale::ONE, WidgetKind::Media, |shell| {
        shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
            title: String::from("Track"),
            artist: None,
            artwork_path: None,
            fetched_at_unix: 0,
        }));
        shell.now_playing_transition = None;
    });
    verify_widget_change(&mut shell, RenderScale::ONE, WidgetKind::Media, |shell| {
        shell.set_now_playing_snapshot(None);
        shell.now_playing_transition = None;
    });
}

#[test]
fn overlapping_auth_forces_full_frame() {
    let size = FrameSize::new(100, 100);
    let old = WidgetRegions {
        size,
        header: Some(Rect::new(20, 20, 30, 20)),
        media: None,
        indicators: None,
        auth: Some(Rect::new(40, 20, 30, 20)),
        weather: None,
        partial_unsafe: false,
    };
    assert_eq!(
        old.damage_since(old, WidgetKind::Header),
        WidgetDamage::Full
    );
}

#[test]
fn moved_neighbor_or_rotated_backdrop_forces_full_frame() {
    let size = FrameSize::new(500, 400);
    let old = WidgetRegions {
        size,
        header: Some(Rect::new(20, 20, 70, 30)),
        media: Some(Rect::new(20, 300, 200, 50)),
        indicators: None,
        auth: None,
        weather: None,
        partial_unsafe: false,
    };
    assert_eq!(
        WidgetRegions {
            media: Some(Rect::new(20, 290, 200, 60)),
            ..old
        }
        .damage_since(old, WidgetKind::Header),
        WidgetDamage::Full
    );
    assert_eq!(
        WidgetRegions {
            partial_unsafe: true,
            ..old
        }
        .damage_since(old, WidgetKind::Header),
        WidgetDamage::Full
    );
}
