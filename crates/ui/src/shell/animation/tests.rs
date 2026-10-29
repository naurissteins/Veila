use super::*;
use crate::{ClockTheme, DateTheme, ShellKey, ShellTheme};

fn pending_shell() -> (ShellState, Instant) {
    let theme = ShellTheme {
        clock: ClockTheme {
            enabled: false,
            ..ShellTheme::default().clock
        },
        date: DateTheme {
            enabled: false,
            ..ShellTheme::default().date
        },
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.handle_key(ShellKey::Enter);
    let ShellStatus::Pending { started_at, .. } = shell.status else {
        panic!("submitting must start authentication");
    };
    (shell, started_at)
}

#[test]
fn spinner_keeps_advancing_before_and_after_delayed_status() {
    let (mut shell, started_at) = pending_shell();

    for step in 1..=25 {
        let now = started_at + Duration::from_millis(step * 80);
        assert_eq!(
            shell.advance_animated_state_at(now),
            ShellAnimationUpdate::AuthDirty,
            "spinner stalled at step {step}"
        );
        assert_eq!(
            shell.next_animation_in(now),
            Some(if step == 12 {
                Duration::from_millis(40)
            } else {
                Duration::from_millis(80)
            })
        );
        assert!(matches!(
            shell.status,
            ShellStatus::Pending { shown, .. } if shown == (step >= 13)
        ));
        assert_eq!(
            shell.advance_animated_state_at(now),
            ShellAnimationUpdate::None
        );
    }
}

#[test]
fn delayed_status_appears_between_spinner_frames() {
    let (mut shell, started_at) = pending_shell();
    shell.advance_animated_state_at(started_at + Duration::from_millis(960));

    let now = started_at + Duration::from_secs(1);
    assert_eq!(
        shell.advance_animated_state_at(now),
        ShellAnimationUpdate::AuthDirty
    );
    assert_eq!(
        shell.render_context().inline_input_status_text().as_deref(),
        Some("Checking...")
    );
    assert_eq!(
        shell.next_animation_in(now),
        Some(Duration::from_millis(40))
    );
}

#[test]
fn rejected_password_stops_spinner_and_keeps_retry_timer() {
    let (mut shell, started_at) = pending_shell();
    shell.advance_animated_state_at(started_at + Duration::from_millis(80));
    shell.authentication_rejected(Some(2_000), Some(1));
    let ShellStatus::Rejected {
        retry_until: Some(retry_until),
        ..
    } = shell.status
    else {
        panic!("rejected authentication must retain backoff");
    };

    assert!(shell.pending_spinner_phase().is_none());
    assert_eq!(
        shell.next_animation_in(retry_until - Duration::from_millis(1_500)),
        Some(Duration::from_millis(500))
    );
}
