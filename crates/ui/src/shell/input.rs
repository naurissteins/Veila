use std::time::{Duration, Instant};

use super::{ShellAction, ShellKey, ShellState, ShellStatus, avatar::current_retry_seconds};

impl ShellState {
    pub fn handle_key(&mut self, key: ShellKey) -> ShellAction {
        match key {
            ShellKey::Character(character) => {
                if !character.is_control() && self.secret.chars().count() < 128 {
                    self.secret.push(character);
                    self.status = ShellStatus::Idle;
                }
                ShellAction::None
            }
            ShellKey::Backspace => {
                self.secret.pop();
                self.status = ShellStatus::Idle;
                ShellAction::None
            }
            ShellKey::Escape => {
                self.secret.clear();
                self.reveal_secret = false;
                self.reveal_toggle_pressed = false;
                self.status = ShellStatus::Idle;
                ShellAction::None
            }
            ShellKey::Enter => {
                if self.secret.is_empty() {
                    ShellAction::None
                } else {
                    self.status = ShellStatus::Pending;
                    ShellAction::Submit(self.secret.clone())
                }
            }
        }
    }

    pub fn authentication_busy(&mut self) {
        self.status = ShellStatus::Idle;
    }

    pub fn authentication_rejected(&mut self, retry_after_ms: Option<u64>) {
        self.secret.clear();
        self.reveal_secret = false;
        self.reveal_toggle_pressed = false;
        let retry_until = retry_after_ms
            .filter(|retry_after_ms| *retry_after_ms > 0)
            .map(|retry_after_ms| Instant::now() + Duration::from_millis(retry_after_ms));
        let displayed_retry_seconds = retry_until.and_then(current_retry_seconds);
        self.status = ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
        };
    }

    pub fn advance_animated_state(&mut self) -> bool {
        let clock_changed = self.clock.refresh();
        let ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
        } = &mut self.status
        else {
            return clock_changed;
        };

        let next_display = retry_until.and_then(current_retry_seconds);
        if *displayed_retry_seconds == next_display {
            return clock_changed;
        }

        *displayed_retry_seconds = next_display;
        if next_display.is_none() {
            *retry_until = None;
        }

        true
    }
}
