use std::{
    thread,
    time::{Duration, Instant},
};

use veila_renderer::{FrameSize, SoftwareBuffer};

use super::{ShellAction, ShellKey, ShellState, ShellStatus};

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
fn typing_does_not_change_static_scene_revision() {
    let mut shell = ShellState::default();
    let original = shell.static_scene_revision();

    shell.handle_key(ShellKey::Character('a'));

    assert_eq!(shell.static_scene_revision(), original);
}
