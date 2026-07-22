use std::{
    thread,
    time::{Duration, Instant},
};

use veila_common::{
    BackdropMode, BackdropShowWhen, BatterySnapshot, FingerprintStatus, LayerKind,
    NowPlayingSnapshot, PowerAction, WeatherCondition, WeatherSnapshot, WeatherUnit,
};
use veila_common::{
    ClockFormat, DateFormat, HorizontalAlign, InputRevealMode, Secret, StatusDisplayMode,
    VerticalAlign,
};
use veila_renderer::icon::BatteryIcon;
use veila_renderer::{FrameSize, SoftwareBuffer};

use super::{ShellAction, ShellAnimationUpdate, ShellKey, ShellState, ShellStatus, ShellTheme};
use crate::shell::theme::{
    Backdrop, PowerButton, VisualLayer, WidgetPosition, WidgetPositionTarget,
};

#[test]
fn edits_and_submits_password_text() {
    let mut shell = ShellState::default();

    assert_eq!(
        shell.handle_key(ShellKey::Character('a')),
        ShellAction::None
    );
    assert_eq!(
        shell.handle_key(ShellKey::Character('b')),
        ShellAction::None
    );
    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::from(String::from("ab")))
    );
    assert_eq!(shell.handle_key(ShellKey::Backspace), ShellAction::None);
    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::from(String::from("a")))
    );
}

#[test]
fn select_all_then_typing_replaces_secret() {
    let mut shell = ShellState::default();

    assert_eq!(
        shell.handle_key(ShellKey::Character('a')),
        ShellAction::None
    );
    assert_eq!(
        shell.handle_key(ShellKey::Character('b')),
        ShellAction::None
    );

    assert_eq!(shell.handle_key(ShellKey::SelectAll), ShellAction::None);
    assert!(shell.secret_selected);

    assert_eq!(
        shell.handle_key(ShellKey::Character('z')),
        ShellAction::None
    );
    assert_eq!(shell.secret.expose(), "z");
    assert!(!shell.secret_selected);
}

#[test]
fn select_all_then_backspace_clears_secret() {
    let mut shell = ShellState::default();

    assert_eq!(
        shell.handle_key(ShellKey::Character('a')),
        ShellAction::None
    );
    assert_eq!(
        shell.handle_key(ShellKey::Character('b')),
        ShellAction::None
    );

    assert_eq!(shell.handle_key(ShellKey::SelectAll), ShellAction::None);
    assert!(shell.secret_selected);

    assert_eq!(shell.handle_key(ShellKey::Backspace), ShellAction::None);
    assert!(shell.secret.is_empty());
    assert!(!shell.secret_selected);
}

#[test]
fn select_all_changes_static_scene_revision() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    let original = shell.static_scene_revision();

    assert_eq!(shell.handle_key(ShellKey::SelectAll), ShellAction::None);

    assert!(shell.secret_selected);
    assert!(shell.static_scene_revision() > original);
}

#[test]
fn ctrl_u_style_clear_empties_secret_without_hiding_auth() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    shell.handle_key(ShellKey::Character('b'));

    assert_eq!(shell.handle_key(ShellKey::Clear), ShellAction::None);

    assert!(shell.secret.is_empty());
    assert!(shell.input_visible());
    assert!(!shell.secret_selected);
}

#[test]
fn ctrl_u_style_clear_resets_selected_state() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    shell.handle_key(ShellKey::Character('b'));
    shell.handle_key(ShellKey::SelectAll);

    assert_eq!(shell.handle_key(ShellKey::Clear), ShellAction::None);

    assert!(shell.secret.is_empty());
    assert!(!shell.secret_selected);
}

#[test]
fn pointer_press_clears_secret_selection() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    shell.handle_key(ShellKey::SelectAll);

    assert!(shell.secret_selected);
    assert!(shell.handle_pointer_press(1280, 720, 10.0, 10.0));
    assert!(!shell.secret_selected);
}

#[test]
fn empty_enter_submits_authentication() {
    let mut shell = ShellState::default();

    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::new())
    );
    assert!(matches!(
        shell.status,
        ShellStatus::Pending { shown: false, .. }
    ));
}

#[test]
fn emergency_mode_keeps_password_input_flow() {
    let mut shell = ShellState::default();

    shell.activate_emergency();

    assert!(shell.emergency_active());
    assert_eq!(
        shell.handle_key(ShellKey::Character('a')),
        ShellAction::None
    );
    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::from(String::from("a")))
    );
}

#[test]
fn emergency_mode_renders_without_theme_layers() {
    let mut shell = ShellState::new(
        ShellTheme {
            layers: vec![VisualLayer {
                kind: LayerKind::Text,
                text: String::from("rich layer"),
                color: veila_renderer::ClearColor::opaque(255, 255, 255),
                background_color: None,
                font_family: None,
                font_weight: None,
                font_style: None,
                font_size: 48,
                width: None,
                height: None,
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
    let mut buffer = SoftwareBuffer::solid(
        FrameSize::new(800, 450),
        veila_renderer::ClearColor::opaque(0, 0, 0),
    )
    .expect("buffer");

    shell.activate_emergency();
    shell.render(&mut buffer);

    assert!(!shell.has_visual_layers());
    let first_pixel = &buffer.pixels()[0..4];
    assert!(
        buffer
            .pixels()
            .chunks_exact(4)
            .any(|pixel| pixel != first_pixel)
    );
}

#[test]
fn input_reveal_on_interaction_starts_hidden() {
    let shell = ShellState::new(
        ShellTheme {
            input_reveal_on_interaction: true,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    assert!(shell.identity_visible());
    assert!(!shell.input_visible());
    assert_eq!(
        shell.hidden_reveal_hint(),
        Some("Press any key or click to continue")
    );
}

#[test]
fn full_reveal_mode_starts_with_entire_auth_stack_hidden() {
    let shell = ShellState::new(
        ShellTheme {
            input_reveal_on_interaction: true,
            input_reveal_mode: InputRevealMode::Full,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    assert!(!shell.identity_visible());
    assert!(!shell.input_visible());
    assert_eq!(
        shell.hidden_reveal_hint(),
        Some("Press any key or click to continue")
    );
}

#[test]
fn disabled_reveal_hint_stays_hidden_even_when_auth_input_is_hidden() {
    let shell = ShellState::new(
        ShellTheme {
            input_reveal_on_interaction: true,
            reveal_enabled: false,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );

    assert_eq!(shell.hidden_reveal_hint(), None);
}

#[test]
fn first_character_reveals_hidden_auth_stack() {
    let mut shell = ShellState::new(
        ShellTheme {
            input_reveal_on_interaction: true,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let original = shell.static_scene_revision();

    assert_eq!(
        shell.handle_key(ShellKey::Character('a')),
        ShellAction::None
    );

    assert!(shell.identity_visible());
    assert!(shell.input_visible());
    assert_eq!(shell.hidden_reveal_hint(), None);
    assert!(shell.static_scene_revision() > original);
    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::from(String::from("a")))
    );
}

#[test]
fn pointer_motion_does_not_reveal_hidden_auth_stack() {
    let mut shell = ShellState::new(
        ShellTheme {
            input_reveal_on_interaction: true,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let original = shell.static_scene_revision();

    assert!(!shell.handle_pointer_motion(1280, 720, 40.0, 40.0));

    assert!(shell.identity_visible());
    assert!(!shell.input_visible());
    assert_eq!(shell.static_scene_revision(), original);
}

#[test]
fn pointer_press_reveals_hidden_auth_stack() {
    let mut shell = ShellState::new(
        ShellTheme {
            input_reveal_on_interaction: true,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let original = shell.static_scene_revision();

    assert!(shell.handle_pointer_press(1280, 720, 40.0, 40.0));

    assert!(shell.identity_visible());
    assert!(shell.input_visible());
    assert!(shell.static_scene_revision() > original);
}

#[test]
fn escape_rehides_auth_stack_when_enabled() {
    let mut shell = ShellState::new(
        ShellTheme {
            input_reveal_on_interaction: true,
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    shell.handle_key(ShellKey::Character('a'));

    assert_eq!(shell.handle_key(ShellKey::Escape), ShellAction::None);
    assert!(shell.identity_visible());
    assert!(!shell.input_visible());
    assert_eq!(
        shell.hidden_reveal_hint(),
        Some("Press any key or click to continue")
    );
    assert!(shell.secret.is_empty());
}

#[test]
fn delayed_pending_state_becomes_visible_after_timeout() {
    let mut shell = ShellState::default();

    assert_eq!(
        shell.handle_key(ShellKey::Character('a')),
        ShellAction::None
    );
    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::from(String::from("a")))
    );

    thread::sleep(Duration::from_millis(1_050));
    let update = shell.advance_animated_state_update();
    assert!(
        matches!(
            update,
            ShellAnimationUpdate::AuthDirty | ShellAnimationUpdate::Full
        ),
        "{update:?}"
    );
    assert!(matches!(
        shell.status,
        ShellStatus::Pending { shown: true, .. }
    ));
}

#[test]
fn pending_state_requests_active_animation_polling() {
    let mut shell = ShellState::default();

    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::new())
    );

    assert_eq!(shell.animation_poll_interval(), Duration::from_millis(80));
    assert!(shell.pending_spinner_phase().is_some());
}

#[test]
fn pending_state_disables_reveal_toggle_interaction() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('s'));
    let toggle = shell.reveal_toggle_rect_for_frame(1280, 720);

    assert!(shell.handle_pointer_motion(1280, 720, (toggle.x + 2) as f64, (toggle.y + 2) as f64));
    assert!(shell.reveal_toggle_hovered);

    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::from(String::from("s")))
    );
    assert!(shell.handle_pointer_motion(1280, 720, (toggle.x + 2) as f64, (toggle.y + 2) as f64));
    assert!(!shell.reveal_toggle_hovered);
    assert!(!shell.reveal_toggle_pressed);
}

#[test]
fn pending_inline_status_text_uses_short_copy_after_delay() {
    let mut shell = ShellState::default();

    let _ = shell.handle_key(ShellKey::Character('a'));
    let _ = shell.handle_key(ShellKey::Enter);
    thread::sleep(Duration::from_millis(1_050));

    let update = shell.advance_animated_state_update();
    assert!(
        matches!(
            update,
            ShellAnimationUpdate::AuthDirty | ShellAnimationUpdate::Full
        ),
        "{update:?}"
    );
    assert_eq!(
        shell.inline_input_status_text().as_deref(),
        Some("Checking...")
    );
}

#[test]
fn explicit_input_position_keeps_inline_status_when_mode_is_inline() {
    let mut shell = ShellState::new(
        ShellTheme {
            input_position: Some(crate::shell::theme::WidgetPosition {
                halign: HorizontalAlign::Center,
                valign: VerticalAlign::Bottom,
                x: 0,
                y: -64,
                target: crate::shell::theme::WidgetPositionTarget::Screen,
            }),
            ..ShellTheme::default()
        },
        None,
        None,
        false,
    );

    let _ = shell.handle_key(ShellKey::Character('a'));
    let _ = shell.handle_key(ShellKey::Enter);
    thread::sleep(Duration::from_millis(1_050));

    assert!(shell.advance_animated_state());
    assert_eq!(
        shell.inline_input_status_text().as_deref(),
        Some("Checking...")
    );
}

#[test]
fn external_status_mode_disables_inline_status_text() {
    let mut shell = ShellState::new(
        ShellTheme {
            status_mode: StatusDisplayMode::External,
            ..ShellTheme::default()
        },
        None,
        None,
        false,
    );

    let _ = shell.handle_key(ShellKey::Character('a'));
    let _ = shell.handle_key(ShellKey::Enter);
    thread::sleep(Duration::from_millis(1_050));

    assert!(shell.advance_animated_state());
    assert_eq!(shell.inline_input_status_text(), None);
    assert_eq!(
        shell.status_text().as_deref(),
        Some("Checking authentication")
    );
}

#[test]
fn hidden_status_mode_suppresses_auth_feedback() {
    let mut shell = ShellState::new(
        ShellTheme {
            status_mode: StatusDisplayMode::Hidden,
            ..ShellTheme::default()
        },
        None,
        None,
        false,
    );

    let _ = shell.handle_key(ShellKey::Character('a'));
    let _ = shell.handle_key(ShellKey::Enter);
    thread::sleep(Duration::from_millis(1_050));

    assert!(shell.advance_animated_state());
    assert_eq!(shell.inline_input_status_text(), None);
    assert_eq!(
        shell.status_text().as_deref(),
        Some("Checking authentication")
    );
}

#[test]
fn rejection_clears_secret() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    shell.authentication_rejected(Some(1_000), Some(1));

    assert_eq!(shell.handle_key(ShellKey::Enter), ShellAction::None);
}

#[test]
fn rejected_state_changes_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    shell.authentication_rejected(Some(1_000), Some(1));

    assert!(shell.static_scene_revision() > original);
}

#[test]
fn leaving_rejected_state_changes_static_scene_revision() {
    let mut shell = ShellState::default();

    shell.authentication_rejected(Some(1_000), Some(1));
    let rejected_revision = shell.static_scene_revision();
    std::thread::sleep(std::time::Duration::from_millis(1_100));

    assert!(shell.advance_animated_state());
    assert!(shell.static_scene_revision() > rejected_revision);
}

#[test]
fn countdown_state_advances_after_timeout() {
    let mut shell = ShellState {
        status: ShellStatus::Rejected {
            retry_until: Some(Instant::now() + Duration::from_millis(1_100)),
            displayed_retry_seconds: Some(2),
            failed_attempts: Some(2),
        },
        ..ShellState::default()
    };
    thread::sleep(Duration::from_millis(250));

    assert!(shell.advance_animated_state());
}

#[test]
fn rejected_inline_status_text_uses_retry_copy() {
    let mut shell = ShellState::default();

    shell.authentication_rejected(Some(3_000), Some(1));

    assert_eq!(
        shell.inline_input_status_text().as_deref(),
        Some("Try again in 3s")
    );
}

#[test]
fn typing_during_retry_cooldown_preserves_retry_status() {
    let mut shell = ShellState::default();

    shell.authentication_rejected(Some(3_000), Some(1));
    let _ = shell.handle_key(ShellKey::Character('a'));

    assert_eq!(
        shell.inline_input_status_text().as_deref(),
        Some("Try again in 3s")
    );
    assert_eq!(shell.handle_key(ShellKey::Enter), ShellAction::None);
}

#[test]
fn retry_cooldown_clears_rejected_state_after_timeout() {
    let mut shell = ShellState::default();

    shell.authentication_rejected(Some(1_000), Some(1));
    let _ = shell.handle_key(ShellKey::Character('a'));
    thread::sleep(Duration::from_millis(1_100));

    assert!(shell.advance_animated_state());
    assert_eq!(shell.inline_input_status_text(), None);
    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(Secret::from(String::from("a")))
    );
}

#[test]
fn rejected_status_text_includes_failed_attempt_count() {
    let mut shell = ShellState::default();

    shell.authentication_rejected(None, Some(2));

    assert_eq!(
        shell.status_text().as_deref(),
        Some("Authentication failed (2 failed attempts)")
    );
}

#[test]
fn rejected_status_text_includes_retry_and_failed_attempt_count() {
    let mut shell = ShellState::default();

    shell.authentication_rejected(Some(1_000), Some(1));

    assert_eq!(
        shell.status_text().as_deref(),
        Some("Authentication failed (1 failed attempt), retry in 1s")
    );
}

#[test]
fn renders_non_empty_scene() {
    let mut shell = ShellState::default();
    shell.set_focus(true);
    let mut buffer = SoftwareBuffer::new(FrameSize::new(480, 320)).expect("buffer");
    shell.render(&mut buffer);

    assert!(buffer.pixels().iter().any(|byte| *byte != 0));
}

#[test]
fn starts_visually_focused() {
    let shell = ShellState::default();

    assert!(shell.focused);
}

#[test]
fn toggles_password_reveal_when_eye_is_pressed() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('s'));
    let toggle = shell.reveal_toggle_rect_for_frame(1280, 720);

    assert!(shell.handle_pointer_motion(1280, 720, (toggle.x + 2) as f64, (toggle.y + 2) as f64,));
    assert!(shell.reveal_toggle_hovered);
    assert!(shell.handle_pointer_press(1280, 720, (toggle.x + 2) as f64, (toggle.y + 2) as f64,));
    assert!(shell.reveal_toggle_pressed);
    assert!(shell.handle_pointer_release(1280, 720, (toggle.x + 2) as f64, (toggle.y + 2) as f64,));
    assert!(shell.reveal_secret);
}

#[test]
fn clears_hover_state_when_pointer_leaves_toggle() {
    let mut shell = ShellState::default();
    let toggle = shell.reveal_toggle_rect_for_frame(1280, 720);
    shell.handle_pointer_motion(1280, 720, (toggle.x + 2) as f64, (toggle.y + 2) as f64);

    assert!(shell.handle_pointer_leave());
    assert!(!shell.reveal_toggle_hovered);
    assert!(!shell.reveal_toggle_pressed);
}

#[test]
fn power_button_release_requests_unconfirmed_action() {
    let mut shell = ShellState::new(
        power_button_theme(PowerAction::Suspend, false),
        None,
        None,
        true,
    );
    let rect = shell
        .power_button_rect(FrameSize::new(1280, 720), PowerAction::Suspend)
        .expect("power button rect");

    assert!(shell.handle_pointer_press(1280, 720, (rect.x + 2) as f64, (rect.y + 2) as f64));
    assert!(shell.handle_pointer_release(1280, 720, (rect.x + 2) as f64, (rect.y + 2) as f64));
    assert_eq!(
        shell.take_pointer_action(),
        ShellAction::Power(PowerAction::Suspend)
    );
}

#[test]
fn power_button_confirmation_requires_second_click() {
    let mut shell = ShellState::new(
        power_button_theme(PowerAction::Poweroff, true),
        None,
        None,
        true,
    );
    let rect = shell
        .power_button_rect(FrameSize::new(1280, 720), PowerAction::Poweroff)
        .expect("power button rect");

    assert!(shell.handle_pointer_press(1280, 720, (rect.x + 2) as f64, (rect.y + 2) as f64));
    assert!(shell.handle_pointer_release(1280, 720, (rect.x + 2) as f64, (rect.y + 2) as f64));
    assert_eq!(shell.take_pointer_action(), ShellAction::None);
    assert_eq!(
        shell.power_confirmation_action(),
        Some(PowerAction::Poweroff)
    );

    assert!(shell.handle_pointer_press(1280, 720, (rect.x + 2) as f64, (rect.y + 2) as f64));
    assert!(shell.handle_pointer_release(1280, 720, (rect.x + 2) as f64, (rect.y + 2) as f64));
    assert_eq!(
        shell.take_pointer_action(),
        ShellAction::Power(PowerAction::Poweroff)
    );
}

#[test]
fn can_disable_username_label() {
    let shell = ShellState::new(Default::default(), None, None, false);

    assert!(shell.username_text.is_none());
}

fn power_button_theme(action: PowerAction, confirm: bool) -> ShellTheme {
    let mut theme = ShellTheme::default();
    theme.power_buttons = theme.power_buttons.map(|mut button| {
        button.enabled = false;
        button
    });
    theme.power_buttons[0] = PowerButton {
        action,
        enabled: true,
        position: Some(WidgetPosition {
            halign: HorizontalAlign::Left,
            valign: VerticalAlign::Top,
            x: 20,
            y: 20,
            target: WidgetPositionTarget::Screen,
        }),
        background_color: veila_renderer::ClearColor::rgba(0, 0, 0, 80),
        background_size: Some(44),
        radius: Some(22),
        color: Some(veila_renderer::ClearColor::opaque(255, 255, 255)),
        size: 20,
        confirm,
    };
    theme
}

#[test]
fn power_status_text_updates_without_touching_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    assert!(shell.set_power_status_text(Some(String::from("Off in 10s"))));
    assert_eq!(shell.static_scene_revision(), original);
    assert!(!shell.set_power_status_text(Some(String::from("Off in 10s"))));
    assert!(shell.set_power_status_text(None));
    assert_eq!(shell.static_scene_revision(), original);
}

#[test]
fn fingerprint_status_uses_auth_status_text() {
    let mut shell = ShellState::default();

    assert!(shell.set_fingerprint_status(Some(FingerprintStatus::Ready)));
    assert_eq!(
        shell.status_text().as_deref(),
        Some("Touch fingerprint reader")
    );
    assert_eq!(shell.inline_input_status_text(), None);
    assert!(!shell.set_fingerprint_status(Some(FingerprintStatus::Ready)));
    assert!(shell.set_fingerprint_status(None));
    assert_eq!(shell.status_text(), None);
}

#[test]
fn fingerprint_status_does_not_replace_password_placeholder() {
    let mut shell = ShellState::default();

    assert!(shell.set_fingerprint_status(Some(FingerprintStatus::Unavailable)));
    assert_eq!(shell.inline_input_status_text(), None);
    shell.handle_key(ShellKey::Character('a'));

    assert_eq!(shell.inline_input_status_text(), None);
}

#[test]
fn uses_configured_username_override() {
    let shell = ShellState::new_with_username(
        Default::default(),
        None,
        Some(String::from("guest")),
        None,
        true,
    );

    assert_eq!(shell.username_text.as_deref(), Some("guest"));
}

#[test]
fn focus_changes_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    shell.set_focus(false);

    assert!(shell.static_scene_revision() > original);
}

#[test]
fn applying_theme_changes_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    shell.apply_theme(Default::default(), None, None, true);

    assert!(shell.static_scene_revision() > original);
}

#[test]
fn applying_theme_updates_clock_format() {
    let mut shell = ShellState::default();
    let theme = ShellTheme {
        clock_format: ClockFormat::TwelveHour,
        date_format: DateFormat::Iso,
        ..ShellTheme::default()
    };

    shell.apply_theme(theme.clone(), None, None, true);

    assert_eq!(shell.theme.clock_format, ClockFormat::TwelveHour);
    assert_eq!(shell.theme.date_format, DateFormat::Iso);
    assert_eq!(
        shell.clock,
        super::clock::ClockState::current(theme.clock_format, theme.date_format)
    );
}

#[test]
fn typing_refreshes_static_scene_when_placeholder_visibility_changes() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    shell.handle_key(ShellKey::Character('a'));

    assert!(shell.static_scene_revision() > original);
}

#[test]
fn typing_more_text_keeps_static_scene_revision() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    let after_first_character = shell.static_scene_revision();

    shell.handle_key(ShellKey::Character('b'));

    assert_eq!(shell.static_scene_revision(), after_first_character);
}

#[test]
fn clearing_text_refreshes_static_scene_for_placeholder() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    let after_first_character = shell.static_scene_revision();

    shell.handle_key(ShellKey::Backspace);

    assert!(shell.static_scene_revision() > after_first_character);
}

#[test]
fn caps_lock_toggle_does_not_change_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    assert!(shell.set_caps_lock_active(true));
    assert_eq!(shell.static_scene_revision(), original);
    assert!(shell.caps_lock_active);
    assert!(!shell.set_caps_lock_active(true));
}

#[test]
fn keyboard_layout_toggle_does_not_change_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    assert!(shell.set_keyboard_layout_label(Some(String::from("US"))));
    assert_eq!(shell.static_scene_revision(), original);
    assert_eq!(shell.keyboard_layout_label.as_deref(), Some("US"));
    assert!(!shell.set_keyboard_layout_label(Some(String::from("US"))));
}

#[test]
fn weather_widget_requires_location_and_snapshot() {
    let shell = ShellState::new_with_username_and_weather(
        Default::default(),
        None,
        None,
        None,
        true,
        Some(String::from("Riga")),
        None,
        WeatherUnit::Celsius,
        None,
    );

    assert!(shell.weather.is_none());
}

#[test]
fn weather_widget_uses_snapshot_data() {
    let shell = ShellState::new_with_username_and_weather(
        Default::default(),
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

    let weather = shell.weather.as_ref().expect("weather widget");
    assert_eq!(weather.location, "Riga");
    assert_eq!(weather.temperature_text, "7°C");
}

#[test]
fn weather_widget_formats_fahrenheit_when_configured() {
    let shell = ShellState::new_with_username_and_weather(
        Default::default(),
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
        WeatherUnit::Fahrenheit,
        None,
    );

    let weather = shell.weather.as_ref().expect("weather widget");
    assert_eq!(weather.temperature_text, "45°F");
}

#[test]
fn now_playing_widget_uses_snapshot_data() {
    let shell = ShellState::new_with_username_and_widgets(
        Default::default(),
        None,
        None,
        None,
        true,
        None,
        None,
        WeatherUnit::Celsius,
        None,
        Some(NowPlayingSnapshot {
            title: String::from("Northern Attitude"),
            artist: Some(String::from("Noah Kahan")),
            artwork_path: None,
            fetched_at_unix: 0,
        }),
    );

    let now_playing = shell.now_playing.as_ref().expect("now playing widget");
    assert_eq!(now_playing.title, "Northern Attitude");
    assert_eq!(now_playing.artist.as_deref(), Some("Noah Kahan"));
}

#[test]
fn battery_widget_uses_snapshot_data() {
    let shell = ShellState::new_with_username_and_weather(
        Default::default(),
        None,
        None,
        None,
        true,
        None,
        None,
        WeatherUnit::Celsius,
        Some(BatterySnapshot {
            percent: 84,
            charging: false,
        }),
    );

    let battery = shell.battery.as_ref().expect("battery widget");
    assert_eq!(battery.icon, BatteryIcon::Full);
}

#[test]
fn battery_widget_uses_charging_icon_when_charging() {
    let shell = ShellState::new_with_username_and_weather(
        Default::default(),
        None,
        None,
        None,
        true,
        None,
        None,
        WeatherUnit::Celsius,
        Some(BatterySnapshot {
            percent: 12,
            charging: true,
        }),
    );

    let battery = shell.battery.as_ref().expect("battery widget");
    assert_eq!(battery.icon, BatteryIcon::Charging);
}

#[test]
fn updating_now_playing_snapshot_starts_transition_without_static_scene_revision_change() {
    let mut shell = ShellState::new(
        ShellTheme {
            backdrops: Vec::new(),
            ..ShellTheme::default()
        },
        None,
        None,
        true,
    );
    let original = shell.static_scene_revision();

    shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
        title: String::from("Track"),
        artist: Some(String::from("Artist")),
        artwork_path: None,
        fetched_at_unix: 1,
    }));

    assert_eq!(shell.static_scene_revision(), original);
    assert!(shell.now_playing_transition.is_some());
    assert_eq!(
        shell
            .now_playing
            .as_ref()
            .map(|widget| widget.title.as_str()),
        Some("Track")
    );
}

#[test]
fn conditional_now_playing_backdrop_appearing_keeps_static_scene_revision() {
    let mut shell = ShellState::new(
        ShellTheme {
            now_playing_enabled: true,
            backdrops: vec![Backdrop {
                mode: BackdropMode::Solid,
                show_when: BackdropShowWhen::NowPlaying,
                color: veila_renderer::ClearColor::opaque(0, 0, 0),
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
                rotate: 0,
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
    let original = shell.static_scene_revision();

    shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
        title: String::from("Track"),
        artist: Some(String::from("Artist")),
        artwork_path: None,
        fetched_at_unix: 1,
    }));

    assert_eq!(shell.static_scene_revision(), original);
}

#[test]
fn static_scene_cache_variant_ignores_conditional_backdrop_visibility() {
    let mut shell = ShellState::new(
        ShellTheme {
            now_playing_enabled: true,
            backdrops: vec![Backdrop {
                mode: BackdropMode::Solid,
                show_when: BackdropShowWhen::NowPlaying,
                color: veila_renderer::ClearColor::opaque(0, 0, 0),
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
                rotate: 0,
                position: WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Center,
                    x: 0,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                },
                z: 0,
            }],
            layers: vec![VisualLayer {
                kind: LayerKind::Text,
                text: String::from("test"),
                color: veila_renderer::ClearColor::opaque(255, 255, 255),
                background_color: None,
                font_family: None,
                font_weight: None,
                font_style: None,
                font_size: 16,
                width: None,
                height: None,
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
    let hidden_variant = shell
        .static_scene_cache_variant(1)
        .expect("static scene variant");

    shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
        title: String::from("Track"),
        artist: Some(String::from("Artist")),
        artwork_path: None,
        fetched_at_unix: 1,
    }));

    let visible_variant = shell
        .static_scene_cache_variant(1)
        .expect("static scene variant");
    assert_eq!(visible_variant, hidden_variant);
}

#[test]
fn backdrop_cache_variant_includes_rotation() {
    let mut theme = ShellTheme {
        backdrops: vec![Backdrop {
            mode: BackdropMode::Solid,
            show_when: BackdropShowWhen::Always,
            color: veila_renderer::ClearColor::opaque(0, 0, 0),
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
            rotate: 0,
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
    let unrotated = ShellState::new(theme.clone(), None, None, true).backdrop_cache_variant();

    theme.backdrops[0].rotate = 12;
    let rotated = ShellState::new(theme, None, None, true).backdrop_cache_variant();

    assert_ne!(unrotated, rotated);
}

#[test]
fn now_playing_transition_clears_after_fade_duration() {
    let mut shell = ShellState::default();
    shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
        title: String::from("Track"),
        artist: Some(String::from("Artist")),
        artwork_path: None,
        fetched_at_unix: 1,
    }));

    assert!(shell.now_playing_transition.is_some());
    thread::sleep(Duration::from_millis(500));

    assert!(shell.advance_animated_state());
    assert!(shell.now_playing_transition.is_none());
}

#[test]
fn conditional_now_playing_backdrop_disappearing_after_fade_keeps_static_scene_revision() {
    let mut shell = ShellState::new_with_username_and_widgets(
        ShellTheme {
            now_playing_enabled: true,
            now_playing_fade_duration_ms: Some(10),
            backdrops: vec![Backdrop {
                mode: BackdropMode::Solid,
                show_when: BackdropShowWhen::NowPlaying,
                color: veila_renderer::ClearColor::opaque(0, 0, 0),
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
                rotate: 0,
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
            fetched_at_unix: 1,
        }),
    );

    shell.set_now_playing_snapshot(None);
    let after_clear = shell.static_scene_revision();
    thread::sleep(Duration::from_millis(20));

    assert!(shell.advance_animated_state());
    assert_eq!(shell.static_scene_revision(), after_clear);
}

#[test]
fn now_playing_transition_uses_configured_fade_duration() {
    let theme = ShellTheme {
        now_playing_fade_duration_ms: Some(10),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
        title: String::from("Track"),
        artist: Some(String::from("Artist")),
        artwork_path: None,
        fetched_at_unix: 1,
    }));

    assert!(shell.now_playing_transition.is_some());
    thread::sleep(Duration::from_millis(20));

    assert!(shell.advance_animated_state());
    assert!(shell.now_playing_transition.is_none());
}

#[test]
fn now_playing_transition_requests_active_animation_polling() {
    let mut shell = ShellState::default();
    shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
        title: String::from("Track"),
        artist: Some(String::from("Artist")),
        artwork_path: None,
        fetched_at_unix: 1,
    }));

    assert_eq!(shell.animation_poll_interval(), Duration::from_millis(80));
}
