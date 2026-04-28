use std::{
    thread,
    time::{Duration, Instant},
};

use veila_common::ClockFormat;
use veila_common::{
    BatterySnapshot, NowPlayingSnapshot, WeatherCondition, WeatherSnapshot, WeatherUnit,
};
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
fn rejection_clears_secret() {
    let mut shell = ShellState::default();
    shell.handle_key(ShellKey::Character('a'));
    shell.authentication_rejected(Some(1_000));

    assert_eq!(shell.handle_key(ShellKey::Enter), ShellAction::None);
}

#[test]
fn countdown_state_advances_after_timeout() {
    let mut shell = ShellState {
        status: ShellStatus::Rejected {
            retry_until: Some(Instant::now() + Duration::from_millis(1_100)),
            displayed_retry_seconds: Some(2),
        },
        ..ShellState::default()
    };
    thread::sleep(Duration::from_millis(250));

    assert!(shell.advance_animated_state());
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
