use std::time::{Duration, Instant};

/// Milliseconds elapsed since `started_at`, saturating instead of overflowing
pub fn elapsed_ms(started_at: Instant) -> u64 {
    duration_ms(started_at.elapsed())
}

/// Microseconds elapsed since `started_at`, saturating instead of overflowing
pub fn elapsed_us(started_at: Instant) -> u64 {
    duration_us(started_at.elapsed())
}

pub fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

pub fn duration_us(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

/// Milliseconds between an optional start and a known end, saturating if they are out of order
pub fn duration_ms_between(started_at: Option<Instant>, ended_at: Instant) -> Option<u64> {
    started_at.map(|started_at| duration_ms(ended_at.saturating_duration_since(started_at)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{duration_ms, duration_us};

    #[test]
    fn converts_durations_to_whole_units() {
        assert_eq!(duration_ms(Duration::from_millis(1_500)), 1_500);
        assert_eq!(duration_us(Duration::from_millis(2)), 2_000);
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        assert_eq!(duration_ms(Duration::MAX), u64::MAX);
        assert_eq!(duration_us(Duration::MAX), u64::MAX);
    }
}
