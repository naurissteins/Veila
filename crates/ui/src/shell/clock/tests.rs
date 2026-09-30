use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use veila_common::{ClockFormat, ClockStyle, DateFormat};

use super::ClockState;

fn clock_at(datetime: OffsetDateTime) -> ClockState {
    ClockState::from_datetime(datetime, ClockFormat::TwelveHour, DateFormat::Iso)
}

#[test]
fn unchanged_local_minute_keeps_existing_text_buffers() {
    let datetime = OffsetDateTime::UNIX_EPOCH;
    let mut clock = clock_at(datetime);
    let previous = clock.clone();
    let buffers = |clock: &ClockState| {
        [
            clock.time_text.as_ptr(),
            clock.hour_text.as_ptr(),
            clock.minute_text.as_ptr(),
            clock.meridiem_text.as_ref().expect("meridiem").as_ptr(),
            clock.date_text.as_ptr(),
        ]
    };
    let original_buffers = buffers(&clock);

    for seconds in 0..60 {
        assert!(!clock.refresh_at(datetime + Duration::seconds(seconds)));
        assert_eq!(clock, previous);
        assert_eq!(buffers(&clock), original_buffers);
    }
}

#[test]
fn refresh_tracks_forward_and_backward_minute_changes() {
    let datetime = OffsetDateTime::UNIX_EPOCH;
    let mut clock = clock_at(datetime);

    for delta in [1, 12 * 60, 24 * 60, -1, -60, 0] {
        let next = datetime + Duration::minutes(delta);
        assert!(clock.refresh_at(next));
        assert_eq!(clock, clock_at(next));
        assert!(!clock.refresh_at(next + Duration::seconds(59)));
    }
}

#[test]
fn refresh_updates_date_at_leap_day_and_year_rollovers() {
    for (year, month, day) in [(2024, Month::February, 28), (2025, Month::December, 31)] {
        let datetime = Date::from_calendar_date(year, month, day)
            .expect("date")
            .with_hms(23, 59, 59)
            .expect("time")
            .assume_utc();
        let mut clock = clock_at(datetime);
        let next = datetime + Duration::seconds(1);

        assert!(clock.refresh_at(next));
        assert_ne!(clock.date_text(), clock_at(datetime).date_text());
        assert_eq!(clock, clock_at(next));
    }
}

#[test]
fn refresh_observes_offset_changes_within_same_utc_minute() {
    let datetime = OffsetDateTime::UNIX_EPOCH + Duration::hours(12);
    let mut clock = clock_at(datetime);

    for hours in [1, -1, 14, -12, 0] {
        let next = datetime.to_offset(UtcOffset::from_hms(hours, 0, 0).expect("offset"));
        assert!(clock.refresh_at(next));
        assert_eq!(clock, clock_at(next));
    }
}

#[test]
fn refresh_tracks_local_minute_with_second_precision_offsets() {
    let offset = UtcOffset::from_hms(0, 0, 30).expect("offset");
    let datetime = OffsetDateTime::UNIX_EPOCH.to_offset(offset);
    let mut clock = clock_at(datetime);

    assert!(!clock.refresh_at(datetime + Duration::seconds(29)));
    assert!(clock.refresh_at(datetime + Duration::seconds(30)));
    assert_eq!(clock.primary_text(ClockStyle::Standard), "12:01");
    assert!(!clock.refresh_at(datetime + Duration::seconds(60)));
}

#[test]
fn refresh_uses_floor_division_before_unix_epoch() {
    let datetime = OffsetDateTime::UNIX_EPOCH - Duration::seconds(60);
    let mut clock = clock_at(datetime);

    assert!(!clock.refresh_at(datetime + Duration::seconds(59)));
    assert!(clock.refresh_at(OffsetDateTime::UNIX_EPOCH));
    assert_eq!(clock, clock_at(OffsetDateTime::UNIX_EPOCH));
}

#[test]
fn refresh_matches_fresh_snapshots_for_all_formats_and_offsets() {
    for format in [ClockFormat::TwelveHour, ClockFormat::TwentyFourHour] {
        for date_format in [
            DateFormat::Long,
            DateFormat::Iso,
            DateFormat::DayMonthYearDots,
            DateFormat::YearMonthDayDots,
            DateFormat::MonthDayYearSlash,
            DateFormat::DayMonthYearSlash,
            DateFormat::Short,
        ] {
            for hours in [-12, 0, 3, 14] {
                let datetime = OffsetDateTime::UNIX_EPOCH
                    .to_offset(UtcOffset::from_hms(hours, 0, 0).expect("offset"));
                let mut clock = ClockState::from_datetime(datetime, format, date_format);
                for minute in 1..=24 * 60 {
                    let next = datetime + Duration::minutes(minute);
                    assert!(clock.refresh_at(next));
                    assert_eq!(clock, ClockState::from_datetime(next, format, date_format));
                    assert!(!clock.refresh_at(next + Duration::seconds(59)));
                }
            }
        }
    }
}

#[test]
fn formats_clock_snapshot_in_24_hour_mode() {
    let datetime = PrimitiveDateTime::new(
        Date::from_calendar_date(2026, Month::March, 24).expect("date"),
        Time::from_hms(9, 5, 0).expect("time"),
    )
    .assume_offset(UtcOffset::UTC);

    let clock = ClockState::from_datetime(datetime, ClockFormat::TwentyFourHour, DateFormat::Long);

    assert_eq!(clock.primary_text(ClockStyle::Standard), "09:05");
    assert_eq!(clock.primary_text(ClockStyle::Stacked), "09");
    assert_eq!(clock.secondary_text(ClockStyle::Stacked), Some("05"));
    assert_eq!(clock.meridiem_text(), None);
    assert_eq!(clock.date_text(), "Tuesday, March 24");
}

#[test]
fn formats_clock_snapshot_in_12_hour_mode() {
    let datetime = PrimitiveDateTime::new(
        Date::from_calendar_date(2026, Month::March, 24).expect("date"),
        Time::from_hms(15, 5, 0).expect("time"),
    )
    .assume_offset(UtcOffset::UTC);

    let clock = ClockState::from_datetime(datetime, ClockFormat::TwelveHour, DateFormat::Long);

    assert_eq!(clock.primary_text(ClockStyle::Standard), "03:05");
    assert_eq!(clock.primary_text(ClockStyle::Stacked), "03");
    assert_eq!(clock.secondary_text(ClockStyle::Stacked), Some("05"));
    assert_eq!(clock.meridiem_text(), Some("PM"));
    assert_eq!(clock.date_text(), "Tuesday, March 24");
}

#[test]
fn formats_normandy_as_12_am() {
    let datetime = PrimitiveDateTime::new(
        Date::from_calendar_date(2026, Month::March, 24).expect("date"),
        Time::from_hms(0, 5, 0).expect("time"),
    )
    .assume_offset(UtcOffset::UTC);

    let clock = ClockState::from_datetime(datetime, ClockFormat::TwelveHour, DateFormat::Long);

    assert_eq!(clock.primary_text(ClockStyle::Standard), "12:05");
    assert_eq!(clock.meridiem_text(), Some("AM"));
}

#[test]
fn formats_date_presets() {
    let datetime = PrimitiveDateTime::new(
        Date::from_calendar_date(2026, Month::May, 13).expect("date"),
        Time::from_hms(9, 5, 0).expect("time"),
    )
    .assume_offset(UtcOffset::UTC);

    let cases = [
        (DateFormat::Long, "Wednesday, May 13"),
        (DateFormat::Iso, "2026-05-13"),
        (DateFormat::DayMonthYearDots, "13.05.2026"),
        (DateFormat::YearMonthDayDots, "2026.05.13"),
        (DateFormat::MonthDayYearSlash, "05/13/2026"),
        (DateFormat::DayMonthYearSlash, "13/05/2026"),
        (DateFormat::Short, "Wed, May 13"),
    ];

    for (format, expected) in cases {
        let clock = ClockState::from_datetime(datetime, ClockFormat::TwentyFourHour, format);
        assert_eq!(clock.date_text(), expected);
    }
}
