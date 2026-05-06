use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use veila_common::BatterySnapshot;
use veila_common::ipc::LockPowerStatusSnapshot;

use crate::adapters::logind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SuspendSkipReason {
    AuthInFlight,
    OnAcPower,
    NoBatteryDetected,
    MediaPlaying,
}

impl SuspendSkipReason {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::AuthInFlight => "auth-in-flight",
            Self::OnAcPower => "on-ac-power",
            Self::NoBatteryDetected => "no-battery-detected",
            Self::MediaPlaying => "media-playing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SuspendDecision {
    Pending,
    Ready,
    Skipped(SuspendSkipReason),
}

#[derive(Debug, Clone)]
pub(super) struct LockedSuspendState {
    delay: Option<Duration>,
    battery_only: bool,
    skip_while_media_playing: bool,
    last_activity_at: Option<Instant>,
    suspend_requested: bool,
    last_reported_skip_reason: Option<SuspendSkipReason>,
}

impl LockedSuspendState {
    pub(super) fn new(
        delay: Option<Duration>,
        battery_only: bool,
        skip_while_media_playing: bool,
    ) -> Self {
        Self {
            delay,
            battery_only,
            skip_while_media_playing,
            last_activity_at: None,
            suspend_requested: false,
            last_reported_skip_reason: None,
        }
    }

    pub(super) fn set_policy(
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
        self.suspend_requested = false;
        self.last_reported_skip_reason = None;
        self.last_activity_at = if !active_lock || delay.is_none() {
            None
        } else {
            self.last_activity_at.or(Some(now))
        };
    }

    pub(super) fn arm(&mut self, now: Instant) {
        if self.delay.is_none() {
            return;
        }

        self.last_activity_at = Some(now);
        self.suspend_requested = false;
        self.last_reported_skip_reason = None;
    }

    pub(super) fn clear(&mut self) {
        self.last_activity_at = None;
        self.suspend_requested = false;
        self.last_reported_skip_reason = None;
    }

    pub(super) fn note_activity(&mut self, now: Instant) {
        if self.delay.is_none() {
            return;
        }

        self.last_activity_at = Some(now);
        self.suspend_requested = false;
        self.last_reported_skip_reason = None;
    }

    pub(super) fn evaluate(
        &self,
        now: Instant,
        active_lock: bool,
        auth_in_flight: bool,
        battery_snapshot: Option<&BatterySnapshot>,
        media_playing: bool,
    ) -> SuspendDecision {
        if !active_lock || self.suspend_requested {
            return SuspendDecision::Pending;
        }

        let Some(delay) = self.delay else {
            return SuspendDecision::Pending;
        };
        let Some(last_activity_at) = self.last_activity_at else {
            return SuspendDecision::Pending;
        };

        let deadline = last_activity_at
            .checked_add(delay)
            .unwrap_or(last_activity_at);
        if now < deadline {
            return SuspendDecision::Pending;
        }

        if auth_in_flight {
            return SuspendDecision::Skipped(SuspendSkipReason::AuthInFlight);
        }

        if self.battery_only {
            match battery_power_state(battery_snapshot) {
                BatteryPowerState::OnBattery => {}
                BatteryPowerState::Charging => {
                    return SuspendDecision::Skipped(SuspendSkipReason::OnAcPower);
                }
                BatteryPowerState::Unavailable => {
                    return SuspendDecision::Skipped(SuspendSkipReason::NoBatteryDetected);
                }
            }
        }

        if self.skip_while_media_playing && media_playing {
            return SuspendDecision::Skipped(SuspendSkipReason::MediaPlaying);
        }

        SuspendDecision::Ready
    }

    pub(super) fn note_skip_reason(
        &mut self,
        reason: SuspendSkipReason,
    ) -> Option<SuspendSkipReason> {
        if self.last_reported_skip_reason == Some(reason) {
            return None;
        }

        self.last_reported_skip_reason = Some(reason);
        Some(reason)
    }

    pub(super) fn clear_reported_skip_reason(&mut self) {
        self.last_reported_skip_reason = None;
    }

    pub(super) fn mark_requested(&mut self) {
        self.suspend_requested = true;
        self.last_reported_skip_reason = None;
    }

    pub(super) fn power_status_snapshot(
        &self,
        now: Instant,
        active_lock: bool,
    ) -> Option<LockPowerStatusSnapshot> {
        if !active_lock || self.suspend_requested {
            return None;
        }

        let delay = self.delay?;
        let last_activity_at = self.last_activity_at?;
        let deadline = last_activity_at
            .checked_add(delay)
            .unwrap_or(last_activity_at);
        if now >= deadline {
            return None;
        }

        Some(LockPowerStatusSnapshot {
            suspend_remaining_seconds: remaining_seconds(deadline.saturating_duration_since(now)),
        })
    }
}

pub(super) fn suspend_delay_seconds(config: &veila_common::AppConfig) -> Option<u64> {
    config.lock.suspend_seconds.filter(|seconds| *seconds > 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatteryPowerState {
    OnBattery,
    Charging,
    Unavailable,
}

fn battery_power_state(snapshot: Option<&BatterySnapshot>) -> BatteryPowerState {
    match snapshot {
        Some(snapshot) if !snapshot.charging => BatteryPowerState::OnBattery,
        Some(_) => BatteryPowerState::Charging,
        None => BatteryPowerState::Unavailable,
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

pub(super) async fn request_system_suspend(connection: &zbus::Connection) -> Result<()> {
    let manager = logind::ManagerProxy::new(connection)
        .await
        .context("failed to create logind manager proxy for suspend")?;
    manager
        .suspend(false)
        .await
        .context("failed to request system suspend through logind")
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use veila_common::BatterySnapshot;

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
    fn activity_resets_pending_suspend_request() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, false);
        state.arm(now);
        state.mark_requested();
        state.note_activity(now + Duration::from_secs(6));

        assert_eq!(
            state.evaluate(now + Duration::from_secs(7), true, false, None, false),
            SuspendDecision::Pending
        );
    }

    #[test]
    fn battery_only_policy_requires_discharging_snapshot() {
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
                Some(&BatterySnapshot {
                    percent: 80,
                    charging: true,
                }),
                false,
            ),
            SuspendDecision::Skipped(SuspendSkipReason::OnAcPower)
        );
        assert_eq!(
            state.evaluate(
                now + Duration::from_secs(6),
                true,
                false,
                Some(&BatterySnapshot {
                    percent: 80,
                    charging: false,
                }),
                false,
            ),
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
    fn power_status_snapshot_clears_after_deadline_or_request() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, false);
        state.arm(now);

        assert!(
            state
                .power_status_snapshot(now + Duration::from_secs(5), true)
                .is_none()
        );
        state.arm(now);
        state.mark_requested();
        assert!(
            state
                .power_status_snapshot(now + Duration::from_secs(1), true)
                .is_none()
        );
    }
}
