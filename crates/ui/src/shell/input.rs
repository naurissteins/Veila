use std::time::{Duration, Instant};

use super::{ShellAction, ShellKey, ShellState, ShellStatus, avatar::current_retry_seconds};

const DEFAULT_NOW_PLAYING_FADE_DURATION_MS: u64 = 450;
const PENDING_STATUS_DELAY_MS: u64 = 1_000;
const ACTIVE_ANIMATION_POLL_INTERVAL_MS: u64 = 80;
const IDLE_ANIMATION_POLL_INTERVAL_MS: u64 = 250;

impl ShellState {
    pub fn handle_key(&mut self, key: ShellKey) -> ShellAction {
        match key {
            ShellKey::Escape if !self.input_visible() => ShellAction::None,
            ShellKey::Character(character) => {
                self.reveal_auth();
                if !character.is_control()
                    && (self.secret_selected || self.secret.chars().count() < 128)
                {
                    if self.secret_selected {
                        self.secret.clear();
                        self.set_secret_selected(false);
                    }
                    self.secret.push(character);
                    if !self.retry_cooldown_active() {
                        self.clear_rejected_state();
                        self.status = ShellStatus::Idle;
                    }
                }
                ShellAction::None
            }
            ShellKey::Backspace => {
                self.reveal_auth();
                if self.secret_selected {
                    self.secret.clear();
                    self.set_secret_selected(false);
                } else {
                    self.secret.pop();
                }
                if !self.retry_cooldown_active() {
                    self.clear_rejected_state();
                    self.status = ShellStatus::Idle;
                }
                ShellAction::None
            }
            ShellKey::Escape => {
                self.secret.clear();
                self.set_secret_selected(false);
                self.reveal_secret = false;
                self.reveal_toggle_pressed = false;
                self.status = ShellStatus::Idle;
                self.hide_auth();
                ShellAction::None
            }
            ShellKey::SelectAll => {
                self.reveal_auth();
                self.set_secret_selected(!self.secret.is_empty());
                ShellAction::None
            }
            ShellKey::Enter => {
                self.reveal_auth();
                self.set_secret_selected(false);
                if let ShellStatus::Rejected {
                    retry_until: Some(retry_until),
                    ..
                } = &self.status
                    && Instant::now() < *retry_until
                {
                    return ShellAction::None;
                }

                let started_at = Instant::now();
                self.status = ShellStatus::Pending {
                    started_at,
                    visible_after: started_at + Duration::from_millis(PENDING_STATUS_DELAY_MS),
                    shown: false,
                };
                ShellAction::Submit(self.secret.clone())
            }
        }
    }

    pub fn authentication_busy(&mut self) {
        self.status = ShellStatus::Idle;
    }

    pub fn authentication_rejected(
        &mut self,
        retry_after_ms: Option<u64>,
        failed_attempts: Option<u8>,
    ) {
        if !matches!(self.status, ShellStatus::Rejected { .. }) {
            self.bump_static_scene_revision();
        }
        self.secret.clear();
        self.set_secret_selected(false);
        self.reveal_secret = false;
        self.reveal_toggle_pressed = false;
        let retry_until = retry_after_ms
            .filter(|retry_after_ms| *retry_after_ms > 0)
            .map(|retry_after_ms| Instant::now() + Duration::from_millis(retry_after_ms));
        let displayed_retry_seconds = retry_until.and_then(current_retry_seconds);
        self.status = ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
            failed_attempts,
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
            ..
        } = &mut self.status
        {
            if !*shown && Instant::now() >= *visible_after {
                *shown = true;
            }
            return true;
        }
        let ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
            ..
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
            self.clear_rejected_state();
            self.status = ShellStatus::Idle;
        }

        true
    }

    pub(super) fn pending_spinner_phase(&self) -> Option<u8> {
        let ShellStatus::Pending { started_at, .. } = &self.status else {
            return None;
        };

        Some(
            ((started_at.elapsed().as_millis() / u128::from(ACTIVE_ANIMATION_POLL_INTERVAL_MS)) % 8)
                as u8,
        )
    }

    pub fn animation_poll_interval(&self) -> Duration {
        if matches!(self.status, ShellStatus::Pending { .. })
            || self.now_playing_transition.is_some()
        {
            return Duration::from_millis(ACTIVE_ANIMATION_POLL_INTERVAL_MS);
        }

        Duration::from_millis(IDLE_ANIMATION_POLL_INTERVAL_MS)
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

    fn retry_cooldown_active(&self) -> bool {
        matches!(
            self.status,
            ShellStatus::Rejected {
                retry_until: Some(retry_until),
                ..
            } if Instant::now() < retry_until
        )
    }

    fn clear_rejected_state(&mut self) {
        if matches!(self.status, ShellStatus::Rejected { .. }) {
            self.bump_static_scene_revision();
        }
    }
}
