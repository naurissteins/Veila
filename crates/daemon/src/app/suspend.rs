use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use veila_common::BatterySnapshot;

use crate::adapters::logind;

#[derive(Debug, Clone)]
pub(super) struct LockedSuspendState {
    delay: Option<Duration>,
    battery_only: bool,
    skip_while_media_playing: bool,
    last_activity_at: Option<Instant>,
    suspend_requested: bool,
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
    }

    pub(super) fn clear(&mut self) {
        self.last_activity_at = None;
        self.suspend_requested = false;
    }

    pub(super) fn note_activity(&mut self, now: Instant) {
        if self.delay.is_none() {
            return;
        }

        self.last_activity_at = Some(now);
        self.suspend_requested = false;
    }

    pub(super) fn should_suspend(
        &self,
        now: Instant,
        active_lock: bool,
        auth_in_flight: bool,
        battery_snapshot: Option<&BatterySnapshot>,
        media_playing: bool,
    ) -> bool {
        if !active_lock || auth_in_flight || self.suspend_requested {
            return false;
        }

        if self.battery_only && !on_battery_power(battery_snapshot) {
            return false;
        }

        if self.skip_while_media_playing && media_playing {
            return false;
        }

        let Some(delay) = self.delay else {
            return false;
        };
        let Some(last_activity_at) = self.last_activity_at else {
            return false;
        };

        now >= last_activity_at
            .checked_add(delay)
            .unwrap_or(last_activity_at)
    }

    pub(super) fn mark_requested(&mut self) {
        self.suspend_requested = true;
    }
}

pub(super) fn suspend_delay_seconds(config: &veila_common::AppConfig) -> Option<u64> {
    config.lock.suspend_seconds.filter(|seconds| *seconds > 0)
}

fn on_battery_power(snapshot: Option<&BatterySnapshot>) -> bool {
    snapshot.is_some_and(|snapshot| !snapshot.charging)
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

    use super::LockedSuspendState;

    #[test]
    fn does_not_suspend_while_auth_is_in_flight() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, false);
        state.arm(now);

        assert!(!state.should_suspend(now + Duration::from_secs(6), true, true, None, false));
        assert!(state.should_suspend(now + Duration::from_secs(6), true, false, None, false));
    }

    #[test]
    fn activity_resets_pending_suspend_request() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, false);
        state.arm(now);
        state.mark_requested();
        state.note_activity(now + Duration::from_secs(6));

        assert!(!state.should_suspend(now + Duration::from_secs(7), true, false, None, false));
    }

    #[test]
    fn battery_only_policy_requires_discharging_snapshot() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), true, false);
        state.arm(now);

        assert!(!state.should_suspend(now + Duration::from_secs(6), true, false, None, false));
        assert!(!state.should_suspend(
            now + Duration::from_secs(6),
            true,
            false,
            Some(&BatterySnapshot {
                percent: 80,
                charging: true,
            }),
            false,
        ));
        assert!(state.should_suspend(
            now + Duration::from_secs(6),
            true,
            false,
            Some(&BatterySnapshot {
                percent: 80,
                charging: false,
            }),
            false,
        ));
    }

    #[test]
    fn media_playing_policy_blocks_suspend_when_enabled() {
        let now = Instant::now();
        let mut state = LockedSuspendState::new(Some(Duration::from_secs(5)), false, true);
        state.arm(now);

        assert!(!state.should_suspend(now + Duration::from_secs(6), true, false, None, true));
        assert!(state.should_suspend(now + Duration::from_secs(6), true, false, None, false));
    }
}
