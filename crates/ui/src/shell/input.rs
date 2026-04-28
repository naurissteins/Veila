use std::time::{Duration, Instant};

use super::{ShellAction, ShellKey, ShellState, ShellStatus, avatar::current_retry_seconds};

const DEFAULT_NOW_PLAYING_FADE_DURATION_MS: u64 = 450;
const PENDING_STATUS_DELAY_MS: u64 = 1_000;

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
                if let ShellStatus::Rejected {
                    retry_until: Some(retry_until),
                    ..
                } = &self.status
                    && Instant::now() < *retry_until
                {
                    return ShellAction::None;
                }

                self.status = ShellStatus::Pending {
                    visible_after: Instant::now() + Duration::from_millis(PENDING_STATUS_DELAY_MS),
                    shown: false,
                };
                ShellAction::Submit(self.secret.clone())
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
        let mut changed = self.clock.refresh();
        let fade_duration = self.now_playing_fade_duration();
        if let Some(transition) = self.now_playing_transition.as_ref() {
            changed = true;
            if transition.started_at.elapsed() >= fade_duration {
                self.now_playing_transition = None;
            }
        }
        if let ShellStatus::Pending {
            visible_after,
            shown,
        } = &mut self.status
        {
            if !*shown && Instant::now() >= *visible_after {
                *shown = true;
                changed = true;
            }
            return changed;
        }
        let ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
        } = &mut self.status
        else {
            return changed;
        };

        let next_display = retry_until.and_then(current_retry_seconds);
        if *displayed_retry_seconds == next_display {
            return changed;
        }

        *displayed_retry_seconds = next_display;
        if next_display.is_none() {
            *retry_until = None;
        }

        true
    }

    pub(super) fn now_playing_fade_progress(&self) -> Option<u8> {
        let transition = self.now_playing_transition.as_ref()?;
        let fade_duration = self.now_playing_fade_duration();
        let elapsed = transition.started_at.elapsed();
        let clamped = elapsed.min(fade_duration);
        Some(
            ((clamped.as_millis() * 100) / fade_duration.as_millis()).min(u128::from(u8::MAX))
                as u8,
        )
    }

    fn now_playing_fade_duration(&self) -> Duration {
        Duration::from_millis(
            self.theme
                .now_playing_fade_duration_ms
                .unwrap_or(DEFAULT_NOW_PLAYING_FADE_DURATION_MS)
                .clamp(1, 10_000),
        )
    }
}
