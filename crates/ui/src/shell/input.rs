use std::time::{Duration, Instant};

use super::{
    MAX_SECRET_CHARACTERS, ShellAction, ShellKey, ShellState, ShellStatus,
    avatar::current_retry_seconds,
};

const PENDING_STATUS_DELAY_MS: u64 = 1_000;

impl ShellState {
    pub fn handle_key(&mut self, key: ShellKey) -> ShellAction {
        match key {
            ShellKey::Escape if !self.input_visible() => ShellAction::None,
            ShellKey::Character(character) => {
                let was_empty = self.secret.is_empty();
                self.reveal_auth();
                if !character.is_control()
                    && (self.secret_selected || self.secret.char_count() < MAX_SECRET_CHARACTERS)
                {
                    self.input_limit_reached = false;
                    if self.secret_selected {
                        self.clear_secret();
                        self.set_secret_selected(false);
                    }
                    self.secret.push(character);
                    if !self.retry_cooldown_active() {
                        self.clear_rejected_state();
                        if !matches!(self.status, ShellStatus::Challenge { .. }) {
                            self.status = ShellStatus::Idle;
                        }
                    }
                } else if !character.is_control() {
                    self.input_limit_reached = true;
                }
                self.refresh_on_secret_empty_transition(was_empty);
                ShellAction::None
            }
            ShellKey::Backspace => {
                let was_empty = self.secret.is_empty();
                self.reveal_auth();
                self.input_limit_reached = false;
                if self.secret_selected {
                    self.clear_secret();
                    self.set_secret_selected(false);
                } else {
                    self.secret.pop();
                }
                if !self.retry_cooldown_active() {
                    self.clear_rejected_state();
                    if !matches!(self.status, ShellStatus::Challenge { .. }) {
                        self.status = ShellStatus::Idle;
                    }
                }
                self.refresh_on_secret_empty_transition(was_empty);
                ShellAction::None
            }
            ShellKey::Escape => {
                if matches!(self.status, ShellStatus::Challenge { .. }) {
                    self.clear_secret();
                    self.status = ShellStatus::Pending {
                        started_at: Instant::now(),
                        visible_after: Instant::now(),
                        shown: true,
                        displayed_phase: 0,
                    };
                    return ShellAction::CancelAuthentication;
                }
                let was_empty = self.secret.is_empty();
                self.clear_secret();
                self.set_secret_selected(false);
                self.reveal_secret = false;
                self.reveal_toggle_pressed = false;
                self.status = ShellStatus::Idle;
                self.hide_auth();
                self.refresh_on_secret_empty_transition(was_empty);
                ShellAction::None
            }
            ShellKey::Clear => {
                let was_empty = self.secret.is_empty();
                self.reveal_auth();
                self.clear_secret();
                self.set_secret_selected(false);
                self.reveal_secret = false;
                self.reveal_toggle_pressed = false;
                if !self.retry_cooldown_active() {
                    self.clear_rejected_state();
                    if !matches!(self.status, ShellStatus::Challenge { .. }) {
                        self.status = ShellStatus::Idle;
                    }
                }
                self.refresh_on_secret_empty_transition(was_empty);
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

                self.input_limit_reached = false;
                let started_at = Instant::now();
                self.status = ShellStatus::Pending {
                    started_at,
                    visible_after: started_at + Duration::from_millis(PENDING_STATUS_DELAY_MS),
                    shown: false,
                    displayed_phase: 0,
                };
                self.submitted_secret_len = self.secret.char_count();
                ShellAction::Submit(self.secret.take())
            }
        }
    }

    pub fn authentication_busy(&mut self) {
        self.submitted_secret_len = 0;
        self.reveal_secret = false;
        self.status = ShellStatus::Idle;
    }

    pub fn authentication_challenge(&mut self, text: String, echo: bool) {
        self.clear_secret();
        self.set_secret_selected(false);
        self.reveal_secret = false;
        self.reveal_auth();
        self.status = ShellStatus::Challenge { text, echo };
        self.bump_static_scene_revision();
    }

    pub fn authentication_notice(&mut self, text: String) {
        if !matches!(self.status, ShellStatus::Challenge { .. }) {
            self.status = ShellStatus::Notice { text };
            self.bump_static_scene_revision();
        }
    }

    pub fn challenge_echo_on(&self) -> bool {
        matches!(self.status, ShellStatus::Challenge { echo: true, .. })
    }

    pub fn authentication_rejected(
        &mut self,
        retry_after_ms: Option<u64>,
        failed_attempts: Option<u8>,
    ) {
        self.authentication_rejected_with_message(retry_after_ms, failed_attempts, None);
    }

    pub fn authentication_rejected_with_message(
        &mut self,
        retry_after_ms: Option<u64>,
        failed_attempts: Option<u8>,
        message: Option<String>,
    ) {
        if !matches!(self.status, ShellStatus::Rejected { .. }) {
            self.bump_static_scene_revision();
        }
        self.clear_secret();
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
            message,
        };
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

    pub(super) fn clear_rejected_state(&mut self) {
        if matches!(self.status, ShellStatus::Rejected { .. }) {
            self.bump_static_scene_revision();
        }
    }

    fn clear_secret(&mut self) {
        self.secret.clear();
        self.submitted_secret_len = 0;
        self.input_limit_reached = false;
    }

    pub(super) fn input_limit_message(&self) -> Option<String> {
        (self.input_limit_reached
            && matches!(
                self.status,
                ShellStatus::Idle | ShellStatus::Challenge { .. } | ShellStatus::Notice { .. }
            ))
        .then(|| format!("Maximum {MAX_SECRET_CHARACTERS} characters"))
    }

    pub(super) fn displayed_secret_len(&self) -> usize {
        if matches!(self.status, ShellStatus::Pending { .. }) {
            self.submitted_secret_len
        } else {
            self.secret.char_count()
        }
    }

    fn refresh_on_secret_empty_transition(&mut self, was_empty: bool) {
        if was_empty != self.secret.is_empty() {
            self.bump_static_scene_revision();
        }
    }
}
