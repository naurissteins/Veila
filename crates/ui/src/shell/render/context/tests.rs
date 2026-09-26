use super::*;
use crate::shell::{ShellAction, ShellKey};
use veila_renderer::{ClearColor, FrameSize, SoftwareBuffer};

fn shell() -> ShellState {
    let mut shell = ShellState::default();
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    shell
}

fn buffer(scale: u32) -> SoftwareBuffer {
    SoftwareBuffer::solid(
        FrameSize::new(640 * scale, 360 * scale),
        ClearColor::opaque(0, 0, 0),
    )
    .expect("test buffer")
}

#[test]
fn scaled_auth_draws_reuse_text_layouts_across_outputs() {
    let mut shell = shell();
    shell.handle_key(ShellKey::Character('x'));
    for scale in [1, 2, 3, 2, 1, 3] {
        let mut buffer = buffer(scale);
        for _ in 0..3 {
            shell.auth_dirty_rect_scaled(buffer.size(), scale);
            shell.render_auth_dirty_overlay_scaled(&mut buffer, scale);
        }
        shell.with_render_scale(scale, |context| {
            let cache = context.text_layout_cache.borrow();
            assert_eq!(cache.clock.resolutions, 1);
            assert_eq!(cache.placeholder.resolutions, 1);
            assert!(std::ptr::eq(context.shell, &shell));
        });
    }
    assert_eq!(shell.scaled_render_cache.borrow().entries.len(), 2);
}

#[test]
fn clock_changes_refresh_each_scale_without_rebuilding_the_theme() {
    let mut shell = shell();
    for scale in [2, 3] {
        shell.render_scaled(&mut buffer(scale), scale);
    }
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(1));
    for scale in [2, 3] {
        shell.render_scaled(&mut buffer(scale), scale);
        shell.with_render_scale(scale, |context| {
            assert_eq!(context.text_layout_cache.borrow().clock.resolutions, 2);
        });
    }
}

#[test]
fn resizing_reused_context_matches_a_fresh_render() {
    let reused = shell();
    let fresh = shell();
    reused.render_scaled(&mut buffer(2), 2);
    for size in [FrameSize::new(700, 500), FrameSize::new(1900, 1000)] {
        let mut actual = SoftwareBuffer::solid(size, ClearColor::opaque(0, 0, 0)).unwrap();
        let mut expected = actual.clone();
        reused.render_scaled(&mut actual, 2);
        fresh.render_scaled(&mut expected, 2);
        assert_eq!(actual.pixels(), expected.pixels());
    }
}

#[test]
fn theme_reload_invalidates_all_render_caches() {
    let mut shell = shell();
    for scale in [1, 2, 3] {
        shell.render_scaled(&mut buffer(scale), scale);
    }
    let theme = ShellTheme {
        clock_font_size: Some(36),
        input_width: Some(300),
        ..ShellTheme::default()
    };
    shell.apply_theme(theme.clone(), Some("New placeholder".into()), None, true);
    assert!(shell.scaled_render_cache.borrow().entries.is_empty());
    assert_eq!(shell.text_layout_cache.borrow().clock.resolutions, 0);
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    let mut fresh = ShellState::new(theme, Some("New placeholder".into()), None, true);
    fresh.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    for scale in [1, 2, 3] {
        let mut actual = buffer(scale);
        let mut expected = buffer(scale);
        shell.render_scaled(&mut actual, scale);
        fresh.render_scaled(&mut expected, scale);
        assert_eq!(actual.pixels(), expected.pixels());
    }
}

#[test]
fn scale_cache_is_bounded_and_eviction_preserves_pixels() {
    let shell = shell();
    let mut before = buffer(2);
    shell.render_scaled(&mut before, 2);
    for scale in 3..10 {
        shell.with_render_scale(scale, |_| ());
    }
    assert_eq!(
        shell.scaled_render_cache.borrow().entries.len(),
        MAX_SCALED_CONTEXTS
    );
    assert!(
        shell
            .scaled_render_cache
            .borrow()
            .entries
            .iter()
            .all(|entry| entry.scale != 2)
    );
    let mut after = buffer(2);
    shell.render_scaled(&mut after, 2);
    assert_eq!(before.pixels(), after.pixels());
}

#[test]
fn scaled_rendering_borrows_the_live_secret_and_preserves_submission() {
    let mut shell = shell();
    for key in ['t', 'e', 's', 't'] {
        shell.handle_key(ShellKey::Character(key));
    }
    shell.reveal_secret = true;
    for scale in [1, 2, 3] {
        shell.render_scaled(&mut buffer(scale), scale);
        shell.with_render_scale(scale, |context| {
            assert!(std::ptr::eq(&context.shell.secret, &shell.secret));
            assert_eq!(context.shell.secret.expose(), "test");
        });
    }
    let ShellAction::Submit(secret) = shell.handle_key(ShellKey::Enter) else {
        panic!("expected submission");
    };
    assert_eq!(secret.expose(), "test");
    assert!(shell.secret.is_empty());
}

#[test]
fn emergency_rendering_does_not_create_scaled_theme_caches() {
    let mut shell = shell();
    shell.render_emergency_scaled(&mut buffer(2), 2);
    shell.activate_emergency();
    for scale in [1, 2, 3] {
        let mut actual = buffer(scale);
        let mut expected = buffer(scale);
        shell.render_scaled(&mut actual, scale);
        shell.render_emergency_scaled(&mut expected, scale);
        assert_eq!(actual.pixels(), expected.pixels());
        assert!(shell.auth_dirty_rect_scaled(actual.size(), scale).is_none());
    }
    assert!(shell.scaled_render_cache.borrow().entries.is_empty());
}

#[test]
fn zero_scale_uses_unscaled_rendering() {
    let shell = shell();
    let mut actual = buffer(1);
    let mut expected = buffer(1);
    shell.render_scaled(&mut actual, 0);
    shell.render(&mut expected);
    assert_eq!(actual.pixels(), expected.pixels());
    assert!(shell.scaled_render_cache.borrow().entries.is_empty());
}

#[test]
fn cached_scale_draws_updated_media_metadata() {
    use crate::shell::theme::{WidgetPosition, WidgetPositionTarget};
    use veila_common::{HorizontalAlign, NowPlayingSnapshot, VerticalAlign};

    let theme = ShellTheme {
        now_playing_enabled: true,
        now_playing_title_enabled: true,
        now_playing_title_position: Some(WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Top,
            x: 20,
            y: 20,
            target: WidgetPositionTarget::Screen,
        }),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    let mut previous = buffer(2);
    for title in ["First track", "Next track"] {
        shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
            title: title.into(),
            artist: None,
            artwork_path: None,
            fetched_at_unix: 0,
        }));
        shell.now_playing_transition = None;
        let mut current = buffer(2);
        shell.render_dynamic_overlay_scaled(&mut current, 2);
        shell.with_render_scale(2, |context| {
            let cache = context.text_layout_cache.borrow();
            assert_eq!(cache.now_playing_title.key.as_ref().unwrap().text, title);
        });
        assert_ne!(current.pixels(), previous.pixels());
        previous = current;
    }
}
