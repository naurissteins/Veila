use super::Running;
use crate::record::State;
use anyhow::Result;
use std::{
    process::ExitStatus,
    time::{Duration, Instant},
};
use veila_common::{elapsed_ms, elapsed_us, ipc::LineAccumulator, session};

#[derive(Default)]
pub(super) struct Recovery {
    pub confirmed: bool,
    retries: u32,
    retry_at: Option<Instant>,
}

impl Recovery {
    fn schedule(&mut self) -> Option<Duration> {
        if !self.confirmed && self.retries != 0 {
            return None;
        }
        let delay = Duration::from_secs(match self.retries {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 5,
        });
        self.retries = self.retries.saturating_add(1);
        self.retry_at = Some(Instant::now() + delay);
        Some(delay)
    }
    pub fn due(&self) -> bool {
        self.retry_at.is_some_and(|at| Instant::now() >= at)
    }
    pub fn remaining(&self) -> Option<Duration> {
        self.retry_at
            .map(|at| at.saturating_duration_since(Instant::now()))
    }
}

impl Running {
    pub(super) fn launch(&mut self) -> Result<()> {
        self.recovery.retry_at = None;
        let started = Instant::now();
        let log = self
            .log
            .as_ref()
            .and_then(|log| match log.file.try_clone() {
                Ok(file) => Some(file),
                Err(error) => {
                    tracing::warn!(%error, "optional curtain log unavailable");
                    None
                }
            });
        let result = crate::spawn::curtain(
            &self.options,
            &self.config,
            &mut self.record,
            log,
            &mut self.report,
        );
        self.spawned = started;
        match result {
            Ok((child, channel)) => {
                self.child = Some(child);
                self.channel = Some(channel);
                self.report.curtain_spawn_ms = elapsed_ms(started);
                self.report.curtain_spawn_us = elapsed_us(started);
                Ok(())
            }
            Err(error) if error.is::<crate::spawn::UnresolvedChild>() => Err(error),
            Err(error) => {
                tracing::error!(%error, "curtain spawn failed");
                if self.retry() { Ok(()) } else { Err(error) }
            }
        }
    }

    pub(super) fn exited(&mut self, status: ExitStatus) -> Result<Option<i32>> {
        let code = status.code().unwrap_or(session::EXIT_ABNORMAL);
        // Only this child's authorized exit contract can finish the lock lifetime.
        if code == session::EXIT_UNLOCKED && self.locked {
            self.record.value.state = State::Unlocked;
            self.record.save()?;
            return Ok(Some(code));
        }
        tracing::error!(%status, confirmed = self.recovery.confirmed, "curtain exited without an authorized unlock");
        if code == session::EXIT_REFUSED && self.recovery.confirmed {
            tracing::error!(
                "compositor refused replacement locker; retaining recovery ownership; on Hyprland enable misc:allow_session_lock_restore and check veila doctor; on Sweets disable independent general.lock_restart commands"
            );
        }
        if self.retry() {
            return Ok(None);
        }
        let code = if code == session::EXIT_UNLOCKED {
            session::EXIT_ABNORMAL
        } else {
            code
        };
        self.record.value.state = if code == session::EXIT_REFUSED {
            State::Refused
        } else {
            State::Abnormal
        };
        self.record.save()?;
        if !self.notified {
            self.notify(code.max(1), false)?;
        }
        Ok(Some(code))
    }

    fn retry(&mut self) -> bool {
        let Some(delay) = self.recovery.schedule() else {
            return false;
        };
        self.options.emergency = true;
        self.record.recovered = true;
        self.locked = false;
        self.ready = false;
        self.report.curtain = None;
        self.snapshots = None;
        self.snapshot_line = LineAccumulator::with_limit(session::MAX_FRAME_BYTES);
        self.pending.clear();
        self.written = 0;
        if self.recovery.confirmed {
            self.record.value.state = State::Locked;
        }
        if let Err(error) = self.record.save() {
            tracing::error!(%error, "failed to update recovery record; retaining file lock and retrying");
        }
        tracing::error!(
            delay_ms = delay.as_millis(),
            attempt = self.recovery.retries,
            "scheduling emergency curtain recovery with maximum authentication backoff"
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unconfirmed_activation_gets_only_one_immediate_retry() {
        let mut recovery = Recovery::default();
        assert_eq!(recovery.schedule(), Some(Duration::ZERO));
        assert!(recovery.due());
        assert_eq!(recovery.schedule(), None);
    }
    #[test]
    fn confirmed_lock_retries_forever_at_a_capped_delay_even_after_successful_respawns() {
        let mut recovery = Recovery {
            confirmed: true,
            ..Default::default()
        };
        for seconds in [0, 1, 2, 5, 5, 5] {
            assert_eq!(recovery.schedule(), Some(Duration::from_secs(seconds)));
            recovery.retry_at = None;
        }
        recovery.retries = u32::MAX;
        assert_eq!(recovery.schedule(), Some(Duration::from_secs(5)));
        assert_eq!(recovery.retries, u32::MAX);
    }
    #[test]
    fn confirmation_after_initial_failure_preserves_retry_budget() {
        let mut recovery = Recovery::default();
        recovery.schedule();
        recovery.confirmed = true;
        assert_eq!(recovery.schedule(), Some(Duration::from_secs(1)));
    }
}
