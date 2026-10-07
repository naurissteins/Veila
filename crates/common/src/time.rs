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
    use std::time::{Duration, Instant};

    use super::{duration_ms, duration_ms_between, duration_us};

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

    #[test]
    fn truncates_fractional_units_at_the_boundary() {
        for (nanos, millis, micros) in [
            (999, 0, 0),
            (1_000, 0, 1),
            (999_999, 0, 999),
            (1_000_000, 1, 1_000),
            (1_999_999, 1, 1_999),
        ] {
            let duration = Duration::from_nanos(nanos);
            assert_eq!(duration_ms(duration), millis);
            assert_eq!(duration_us(duration), micros);
        }
    }

    #[test]
    fn saturates_immediately_above_the_integer_limit() {
        assert_eq!(
            duration_ms(Duration::from_millis(u64::MAX - 1)),
            u64::MAX - 1
        );
        assert_eq!(
            duration_us(Duration::from_micros(u64::MAX - 1)),
            u64::MAX - 1
        );
        for extra in [
            Duration::ZERO,
            Duration::from_nanos(999),
            Duration::from_millis(1),
        ] {
            assert_eq!(
                duration_ms(Duration::from_millis(u64::MAX) + extra),
                u64::MAX
            );
            assert_eq!(
                duration_us(Duration::from_micros(u64::MAX) + extra),
                u64::MAX
            );
        }
    }

    #[test]
    fn preserves_missing_and_out_of_order_intervals() {
        let end = Instant::now();
        assert_eq!(duration_ms_between(None, end), None);
        assert_eq!(duration_ms_between(Some(end), end), Some(0));
        assert_eq!(
            duration_ms_between(Some(end + Duration::from_secs(1)), end),
            Some(0)
        );
        assert_eq!(
            duration_ms_between(Some(end), end + Duration::from_micros(1_999)),
            Some(1)
        );
    }
}
