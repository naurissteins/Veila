use time::{Month, OffsetDateTime, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClockState {
    minute_key: i64,
    time_text: String,
    date_text: String,
}

impl ClockState {
    pub(super) fn current() -> Self {
        Self::from_datetime(local_now())
    }

    pub(super) fn refresh(&mut self) -> bool {
        let next = Self::current();
        if *self == next {
            return false;
        }

        *self = next;
        true
    }

    pub(super) fn time_text(&self) -> &str {
        &self.time_text
    }

    pub(super) fn date_text(&self) -> &str {
        &self.date_text
    }

    fn from_datetime(datetime: OffsetDateTime) -> Self {
        Self {
            minute_key: datetime.unix_timestamp().div_euclid(60),
            time_text: format!("{:02}:{:02}", datetime.hour(), datetime.minute()),
            date_text: format!(
                "{}, {} {}",
                weekday_name(datetime.weekday()),
                month_name(datetime.month()),
                datetime.day()
            ),
        }
    }
}

fn local_now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|error| {
        tracing::warn!("failed to resolve local time offset: {error}");
        OffsetDateTime::now_utc()
    })
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

fn month_name(month: Month) -> &'static str {
    match month {
        Month::January => "January",
        Month::February => "February",
        Month::March => "March",
        Month::April => "April",
        Month::May => "May",
        Month::June => "June",
        Month::July => "July",
        Month::August => "August",
        Month::September => "September",
        Month::October => "October",
        Month::November => "November",
        Month::December => "December",
    }
}

#[cfg(test)]
mod tests {
    use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

    use super::ClockState;

    #[test]
    fn formats_clock_snapshot() {
        let datetime = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::March, 24).expect("date"),
            Time::from_hms(9, 5, 0).expect("time"),
        )
        .assume_offset(UtcOffset::UTC);

        let clock = ClockState::from_datetime(datetime);

        assert_eq!(clock.time_text(), "09:05");
        assert_eq!(clock.date_text(), "Tuesday, March 24");
    }
}
