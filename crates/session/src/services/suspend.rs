use std::time::{Duration, Instant};

use veila_common::ipc::LockPowerStatusSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendSkipReason {
    AuthInFlight,
    OnAcPower,
    NoBatteryDetected,
    MediaPlaying,
}

impl SuspendSkipReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthInFlight => "auth-in-flight",
            Self::OnAcPower => "on-ac-power",
            Self::NoBatteryDetected => "no-battery-detected",
            Self::MediaPlaying => "media-playing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendDecision {
    Pending,
    Ready,
    Skipped(SuspendSkipReason),
}

#[derive(Debug, Clone)]
pub struct LockedSuspendState {
    delay: Option<Duration>,
    battery_only: bool,
    skip_while_media_playing: bool,
    countdown_from: Option<Instant>,
    last_reported_skip_reason: Option<SuspendSkipReason>,
}

impl LockedSuspendState {
    pub fn new(
        delay: Option<Duration>,
        battery_only: bool,
        skip_while_media_playing: bool,
    ) -> Self {
        Self {
            delay,
            battery_only,
            skip_while_media_playing,
            countdown_from: None,
            last_reported_skip_reason: None,
        }
    }

    pub fn set_policy(
        &mut self,
        delay: Option<Duration>,
        battery_only: bool,
        skip_while_media_playing: bool,
        now: Instant,
        active_lock: bool,
    ) {
        self.delay = delay;
        self.battery_only = battery_only;
        self.skip_while_media_playing = skip_while_media_playing;
        self.last_reported_skip_reason = None;
        self.countdown_from = if !active_lock || delay.is_none() {
            None
        } else {
            self.countdown_from.or(Some(now))
        };
    }

    pub fn arm(&mut self, now: Instant) {
        if self.delay.is_none() {
            return;
        }

        self.countdown_from = Some(now);
        self.last_reported_skip_reason = None;
    }

    pub fn clear(&mut self) {
        self.countdown_from = None;
        self.last_reported_skip_reason = None;
    }

    pub fn note_activity(&mut self, now: Instant) {
        if self.countdown_from.is_none_or(|start| now >= start) {
            self.arm(now);
        }
    }

    pub fn evaluate(
        &self,
        now: Instant,
        active_lock: bool,
        auth_in_flight: bool,
        on_battery: Option<bool>,
        media_playing: bool,
    ) -> SuspendDecision {
        if !active_lock {
            return SuspendDecision::Pending;
        }

        let Some(delay) = self.delay else {
            return SuspendDecision::Pending;
        };
        let Some(countdown_from) = self.countdown_from else {
            return SuspendDecision::Pending;
        };

        if now.saturating_duration_since(countdown_from) < delay {
            return SuspendDecision::Pending;
        }

        if auth_in_flight {
            return SuspendDecision::Skipped(SuspendSkipReason::AuthInFlight);
        }

        if self.battery_only {
            match on_battery {
                Some(true) => {}
                Some(false) => {
                    return SuspendDecision::Skipped(SuspendSkipReason::OnAcPower);
                }
                None => {
                    return SuspendDecision::Skipped(SuspendSkipReason::NoBatteryDetected);
                }
            }
        }

        if self.skip_while_media_playing && media_playing {
            return SuspendDecision::Skipped(SuspendSkipReason::MediaPlaying);
        }

        SuspendDecision::Ready
    }

    pub fn note_skip_reason(&mut self, reason: SuspendSkipReason) -> Option<SuspendSkipReason> {
        if self.last_reported_skip_reason == Some(reason) {
            return None;
        }

        self.last_reported_skip_reason = Some(reason);
        Some(reason)
    }

    pub fn clear_reported_skip_reason(&mut self) {
        self.last_reported_skip_reason = None;
    }

    pub fn power_status_snapshot(
        &self,
        now: Instant,
        active_lock: bool,
    ) -> Option<LockPowerStatusSnapshot> {
        if !active_lock {
            return None;
        }

        let delay = self.delay?;
        let countdown_from = self.countdown_from?;
        let remaining = delay.checked_sub(now.saturating_duration_since(countdown_from))?;
        if remaining.is_zero() {
            return None;
        }

        Some(LockPowerStatusSnapshot {
            suspend_remaining_seconds: remaining_seconds(remaining),
        })
    }
}

fn remaining_seconds(duration: Duration) -> u64 {
    let seconds = duration.as_secs();
    if duration.subsec_nanos() == 0 {
        seconds.max(1)
    } else {
        seconds.saturating_add(1)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{LockedSuspendState, SuspendDecision, SuspendSkipReason};

    #[test]
    fn does_not_suspend_while_auth_is_in_flight() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, false);
        state.arm(now);

        assert_eq!(
            state.evaluate(now + Duration::from_secs(6), true, true, None, false),
            SuspendDecision::Skipped(SuspendSkipReason::AuthInFlight)
        );
        assert_eq!(
            state.evaluate(now + Duration::from_secs(6), true, false, None, false),
            SuspendDecision::Ready
        );
    }

    #[test]
    fn rearming_starts_a_full_countdown_and_ignores_older_activity() {
        let now = Instant::now();
        let delay = Duration::from_secs(5);
        let mut state = LockedSuspendState::new(Some(delay), false, false);
        state.arm(now);
        assert_eq!(
            state.evaluate(now + delay, true, false, None, false),
            SuspendDecision::Ready
        );

        let rearmed = now + Duration::from_secs(10);
        state.note_activity(rearmed);
        state.note_activity(now + delay);

        assert_eq!(
            state.evaluate(rearmed + Duration::from_secs(4), true, false, None, false),
            SuspendDecision::Pending
        );
        assert_eq!(
            state.evaluate(rearmed + delay, true, false, None, false),
            SuspendDecision::Ready
        );
    }

    #[test]
    fn battery_only_policy_requires_system_battery_power() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), true, false);
        state.arm(now);

        assert_eq!(
            state.evaluate(now + Duration::from_secs(6), true, false, None, false),
            SuspendDecision::Skipped(SuspendSkipReason::NoBatteryDetected)
        );
        assert_eq!(
            state.evaluate(
                now + Duration::from_secs(6),
                true,
                false,
                Some(false),
                false
            ),
            SuspendDecision::Skipped(SuspendSkipReason::OnAcPower)
        );
        assert_eq!(
            state.evaluate(now + Duration::from_secs(6), true, false, Some(true), false),
            SuspendDecision::Ready
        );
    }

    #[test]
    fn media_playing_policy_blocks_suspend_when_enabled() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, true);
        state.arm(now);

        assert_eq!(
            state.evaluate(now + Duration::from_secs(6), true, false, None, true),
            SuspendDecision::Skipped(SuspendSkipReason::MediaPlaying)
        );
        assert_eq!(
            state.evaluate(now + Duration::from_secs(6), true, false, None, false),
            SuspendDecision::Ready
        );
    }

    #[test]
    fn skip_reason_logging_only_reports_reason_changes_once() {
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, true);

        assert_eq!(
            state.note_skip_reason(SuspendSkipReason::MediaPlaying),
            Some(SuspendSkipReason::MediaPlaying)
        );
        assert_eq!(
            state.note_skip_reason(SuspendSkipReason::MediaPlaying),
            None
        );
        assert_eq!(
            state.note_skip_reason(SuspendSkipReason::OnAcPower),
            Some(SuspendSkipReason::OnAcPower)
        );
        state.clear_reported_skip_reason();
        assert_eq!(
            state.note_skip_reason(SuspendSkipReason::OnAcPower),
            Some(SuspendSkipReason::OnAcPower)
        );
    }

    #[test]
    fn power_status_snapshot_counts_down_while_pending() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(20)), false, false);
        state.arm(now);

        assert_eq!(
            state
                .power_status_snapshot(now + Duration::from_secs(1), true)
                .map(|snapshot| snapshot.suspend_remaining_seconds),
            Some(19)
        );
    }

    #[test]
    fn a_delay_beyond_the_instant_range_does_not_become_due_immediately() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::MAX), false, false);
        state.arm(now);

        assert_eq!(
            state.evaluate(now, true, false, None, false),
            SuspendDecision::Pending
        );
        assert_eq!(
            state
                .power_status_snapshot(now, true)
                .map(|status| status.suspend_remaining_seconds),
            Some(u64::MAX),
        );
    }

    #[test]
    fn power_status_snapshot_clears_at_deadline_and_returns_after_rearm() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, false);
        state.arm(now);

        assert!(
            state
                .power_status_snapshot(now + Duration::from_secs(5), true)
                .is_none()
        );
        let rearmed = now + Duration::from_secs(7);
        state.note_activity(rearmed);
        assert_eq!(
            state
                .power_status_snapshot(rearmed, true)
                .map(|status| status.suspend_remaining_seconds),
            Some(5),
        );
    }
}
