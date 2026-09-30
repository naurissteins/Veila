use std::time::Duration;

use tokio::time::{self, Interval, MissedTickBehavior};

const IDLE_INTERVAL: Duration = Duration::from_secs(1);
const ACTIVE_INTERVAL: Duration = Duration::from_millis(250);

pub(super) struct MaintenanceClock {
    interval: Interval,
}

impl MaintenanceClock {
    pub(super) fn new() -> Self {
        Self {
            interval: interval(IDLE_INTERVAL),
        }
    }

    pub(super) fn sync(&mut self, urgent: bool) {
        let period = if urgent {
            ACTIVE_INTERVAL
        } else {
            IDLE_INTERVAL
        };
        if self.interval.period() != period {
            // A state transition gets an immediate tick instead of waiting for the idle deadline.
            self.interval = interval(period);
        }
    }

    pub(super) async fn tick(&mut self) {
        self.interval.tick().await;
    }
}

fn interval(period: Duration) -> Interval {
    let mut interval = time::interval(period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    interval
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn idle_clock_does_not_wake_at_the_active_cadence() {
        let mut clock = MaintenanceClock::new();
        clock.tick().await;
        clock.sync(false);
        assert!(
            time::timeout(Duration::from_millis(400), clock.tick())
                .await
                .is_err()
        );
        time::timeout(Duration::from_secs(1), clock.tick())
            .await
            .expect("idle tick");
    }

    #[tokio::test]
    async fn urgent_work_does_not_wait_for_an_idle_tick() {
        let mut clock = MaintenanceClock::new();
        clock.tick().await;
        clock.sync(true);
        time::timeout(Duration::from_millis(100), clock.tick())
            .await
            .expect("immediate urgent tick");
        clock.sync(true);
        assert!(
            time::timeout(Duration::from_millis(100), clock.tick())
                .await
                .is_err()
        );
        time::timeout(Duration::from_millis(400), clock.tick())
            .await
            .expect("active tick");
    }

    #[tokio::test]
    async fn returning_to_idle_stops_the_fast_ticks() {
        let mut clock = MaintenanceClock::new();
        clock.sync(true);
        clock.tick().await;
        clock.sync(false);
        clock.tick().await;
        assert!(
            time::timeout(Duration::from_millis(400), clock.tick())
                .await
                .is_err()
        );
    }
}
