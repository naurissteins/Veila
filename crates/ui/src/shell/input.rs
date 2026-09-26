use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{
    ShellAction, ShellAnimationUpdate, ShellKey, ShellState, ShellStatus,
    avatar::current_retry_seconds,
};

const DEFAULT_NOW_PLAYING_FADE_DURATION_MS: u64 = 450;
const PENDING_STATUS_DELAY_MS: u64 = 1_000;
const ACTIVE_ANIMATION_POLL_INTERVAL_MS: u64 = 80;

impl ShellState {
    pub fn handle_key(&mut self, key: ShellKey) -> ShellAction {
        match key {
            ShellKey::Escape if !self.input_visible() => ShellAction::None,
            ShellKey::Character(character) => {
                let was_empty = self.secret.is_empty();
                self.reveal_auth();
                if !character.is_control()
                    && (self.secret_selected || self.secret.char_count() < 128)
                {
                    if self.secret_selected {
                        self.clear_secret();
                        self.set_secret_selected(false);
                    }
                    self.secret.push(character);
                    if !self.retry_cooldown_active() {
                        self.clear_rejected_state();
                        self.status = ShellStatus::Idle;
                    }
                }
                self.refresh_on_secret_empty_transition(was_empty);
                ShellAction::None
            }
            ShellKey::Backspace => {
                let was_empty = self.secret.is_empty();
                self.reveal_auth();
                if self.secret_selected {
                    self.clear_secret();
                    self.set_secret_selected(false);
                } else {
                    self.secret.pop();
                }
                if !self.retry_cooldown_active() {
                    self.clear_rejected_state();
                    self.status = ShellStatus::Idle;
                }
                self.refresh_on_secret_empty_transition(was_empty);
                ShellAction::None
            }
            ShellKey::Escape => {
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
                    self.status = ShellStatus::Idle;
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

    pub fn authentication_rejected(
        &mut self,
        retry_after_ms: Option<u64>,
        failed_attempts: Option<u8>,
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
        };
    }

    pub fn advance_animated_state(&mut self) -> bool {
        self.advance_animated_state_update() != ShellAnimationUpdate::None
    }

    pub fn advance_animated_state_update(&mut self) -> ShellAnimationUpdate {
        let mut changed =
            (self.theme.clock_enabled || self.theme.date_enabled) && self.clock.refresh();
        changed |= self.clear_expired_power_confirmation(Instant::now());
        let fade_duration = self.now_playing_fade_duration();
        if let Some(transition) = self.now_playing_transition.as_ref() {
            changed = true;
            if transition.started_at.elapsed() >= fade_duration {
                self.now_playing_transition = None;
            }
        }
        if let ShellStatus::Pending {
            started_at,
            visible_after,
            shown,
            displayed_phase,
            ..
        } = &mut self.status
        {
            let now = Instant::now();
            if !*shown {
                if now < *visible_after {
                    return if changed {
                        ShellAnimationUpdate::Full
                    } else {
                        ShellAnimationUpdate::None
                    };
                }
                *shown = true;
                *displayed_phase = spinner_phase(*started_at, now);
                return if changed {
                    ShellAnimationUpdate::Full
                } else {
                    ShellAnimationUpdate::AuthDirty
                };
            }
            let phase = spinner_phase(*started_at, now);
            let phase_changed = phase != *displayed_phase;
            *displayed_phase = phase;
            return if changed {
                ShellAnimationUpdate::Full
            } else if phase_changed {
                ShellAnimationUpdate::AuthDirty
            } else {
                ShellAnimationUpdate::None
            };
        }
        let ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
            ..
        } = &mut self.status
        else {
            return if changed {
                ShellAnimationUpdate::Full
            } else {
                ShellAnimationUpdate::None
            };
        };

        let next_display = retry_until.and_then(current_retry_seconds);
        if *displayed_retry_seconds == next_display {
            return if changed {
                ShellAnimationUpdate::Full
            } else {
                ShellAnimationUpdate::None
            };
        }

        *displayed_retry_seconds = next_display;
        if next_display.is_none() {
            self.clear_rejected_state();
            self.status = ShellStatus::Idle;
        }

        ShellAnimationUpdate::Full
    }

    pub(super) fn pending_spinner_phase(&self) -> Option<u8> {
        let ShellStatus::Pending { started_at, .. } = &self.status else {
            return None;
        };

        Some(spinner_phase(*started_at, Instant::now()))
    }

    pub fn next_animation_in(&self, now: Instant) -> Option<Duration> {
        let next_minute = (self.theme.clock_enabled || self.theme.date_enabled).then(|| {
            let since_epoch = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            Duration::from_secs(60 - since_epoch.as_secs() % 60)
                .saturating_sub(Duration::from_nanos(u64::from(since_epoch.subsec_nanos())))
        });
        let pending = match self.status {
            ShellStatus::Pending {
                visible_after,
                shown: false,
                ..
            } => Some(visible_after.saturating_duration_since(now)),
            ShellStatus::Pending {
                started_at,
                shown: true,
                ..
            } => {
                let phase_nanos = u128::from(ACTIVE_ANIMATION_POLL_INTERVAL_MS) * 1_000_000;
                let elapsed_nanos = now.saturating_duration_since(started_at).as_nanos();
                Some(Duration::from_nanos(
                    (phase_nanos - elapsed_nanos % phase_nanos) as u64,
                ))
            }
            _ => None,
        };
        let retry = match self.status {
            ShellStatus::Rejected {
                retry_until: Some(retry_until),
                ..
            } => {
                let remaining = retry_until.saturating_duration_since(now);
                let nanosecond_remainder = remaining.subsec_nanos();
                Some(if remaining.is_zero() {
                    Duration::ZERO
                } else if nanosecond_remainder == 0 {
                    Duration::from_secs(1)
                } else {
                    Duration::from_nanos(u64::from(nanosecond_remainder))
                })
            }
            _ => None,
        };
        let fade = self.now_playing_transition.as_ref().map(|transition| {
            (transition.started_at + self.now_playing_fade_duration())
                .saturating_duration_since(now)
                .min(Duration::from_millis(ACTIVE_ANIMATION_POLL_INTERVAL_MS))
        });
        let confirmation = self
            .power_confirmation
            .map(|confirmation| confirmation.expires_at.saturating_duration_since(now));

        [next_minute, pending, retry, fade, confirmation]
            .into_iter()
            .flatten()
            .min()
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

    fn clear_secret(&mut self) {
        self.secret.clear();
        self.submitted_secret_len = 0;
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

fn spinner_phase(started_at: Instant, now: Instant) -> u8 {
    ((now.saturating_duration_since(started_at).as_millis()
        / u128::from(ACTIVE_ANIMATION_POLL_INTERVAL_MS))
        % 8) as u8
}
