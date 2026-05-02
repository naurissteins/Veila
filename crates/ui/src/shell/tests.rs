use std::{
    thread,
    time::{Duration, Instant},
};

use veila_common::{
    BatterySnapshot, NowPlayingSnapshot, WeatherCondition, WeatherSnapshot, WeatherUnit,
};
use veila_common::{ClockFormat, InputRevealMode};
use veila_renderer::icon::BatteryIcon;
use veila_renderer::{FrameSize, SoftwareBuffer};

use super::{ShellAction, ShellKey, ShellState, ShellStatus, ShellTheme};

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
        ShellAction::Submit(String::from("ab"))
    );
    assert_eq!(shell.handle_key(ShellKey::Backspace), ShellAction::None);
    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(String::from("a"))
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
    assert_eq!(shell.secret, "z");
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
fn empty_enter_submits_authentication() {
    let mut shell = ShellState::default();

    assert_eq!(
        shell.handle_key(ShellKey::Enter),
        ShellAction::Submit(String::new())
    );
    assert!(matches!(
        shell.status,
        ShellStatus::Pending { shown: false, .. }
    ));
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
        ShellAction::Submit(String::from("a"))
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
        ShellAction::Submit(String::from("a"))
    );

    thread::sleep(Duration::from_millis(1_050));
    assert!(shell.advance_animated_state());
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
        ShellAction::Submit(String::new())
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
        ShellAction::Submit(String::from("s"))
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

    assert!(shell.advance_animated_state());
    assert_eq!(
        shell.inline_input_status_text().as_deref(),
        Some("Checking...")
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
        ShellAction::Submit(String::from("a"))
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
fn can_disable_username_label() {
    let shell = ShellState::new(Default::default(), None, None, false);

    assert!(shell.username_text.is_none());
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
        ..ShellTheme::default()
    };

    shell.apply_theme(theme.clone(), None, None, true);

    assert_eq!(shell.theme.clock_format, ClockFormat::TwelveHour);
    assert_eq!(
        shell.clock,
        super::clock::ClockState::current(theme.clock_format)
    );
}

#[test]
fn typing_does_not_change_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    shell.handle_key(ShellKey::Character('a'));

    assert_eq!(shell.static_scene_revision(), original);
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
    let mut shell = ShellState::default();
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
