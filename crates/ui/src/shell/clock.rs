use time::{Month, OffsetDateTime, Weekday};
use veila_common::{ClockFormat, ClockStyle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClockState {
    minute_key: i64,
    format: ClockFormat,
    time_text: String,
    hour_text: String,
    minute_text: String,
    meridiem_text: Option<String>,
    date_text: String,
}

impl ClockState {
    pub(super) fn current(format: ClockFormat) -> Self {
        Self::from_datetime(local_now(), format)
    }

    pub(super) fn refresh(&mut self) -> bool {
        let next = Self::current(self.format);
        if *self == next {
            return false;
        }

        *self = next;
        true
    }

    pub(super) fn primary_text(&self, style: ClockStyle) -> &str {
        match style {
            ClockStyle::Standard => &self.time_text,
            ClockStyle::Stacked => &self.hour_text,
        }
    }

    pub(super) fn secondary_text(&self, style: ClockStyle) -> Option<&str> {
        match style {
            ClockStyle::Standard => None,
            ClockStyle::Stacked => Some(&self.minute_text),
        }
    }

    pub(super) fn date_text(&self) -> &str {
        &self.date_text
    }

    pub(super) fn meridiem_text(&self) -> Option<&str> {
        self.meridiem_text.as_deref()
    }

    fn from_datetime(datetime: OffsetDateTime, format: ClockFormat) -> Self {
        let (time_text, hour_text, minute_text, meridiem_text) = format_time(datetime, format);

        Self {
            minute_key: datetime.unix_timestamp().div_euclid(60),
            format,
            time_text,
            hour_text,
            minute_text,
            meridiem_text,
            date_text: format!(
                "{}, {} {}",
                weekday_name(datetime.weekday()),
                month_name(datetime.month()),
                datetime.day()
            ),
        }
    }
}

impl super::ShellState {
    pub fn set_preview_time(&mut self, datetime: OffsetDateTime) {
        self.clock = ClockState::from_datetime(datetime, self.theme.clock_format);
    }
}

fn format_time(
    datetime: OffsetDateTime,
    format: ClockFormat,
) -> (String, String, String, Option<String>) {
    match format {
        ClockFormat::TwentyFourHour => (
            format!("{:02}:{:02}", datetime.hour(), datetime.minute()),
            format!("{:02}", datetime.hour()),
            format!("{:02}", datetime.minute()),
            None,
        ),
        ClockFormat::TwelveHour => {
            let hour = datetime.hour();
            let meridiem = if hour < 12 { "AM" } else { "PM" };
            let display_hour = match hour % 12 {
                0 => 12,
                value => value,
            };

            (
                format!("{display_hour:02}:{:02}", datetime.minute()),
                format!("{display_hour:02}"),
                format!("{:02}", datetime.minute()),
                Some(String::from(meridiem)),
            )
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
    use veila_common::{ClockFormat, ClockStyle};

    use super::ClockState;

    #[test]
    fn formats_clock_snapshot_in_24_hour_mode() {
        let datetime = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::March, 24).expect("date"),
            Time::from_hms(9, 5, 0).expect("time"),
        )
        .assume_offset(UtcOffset::UTC);

        let clock = ClockState::from_datetime(datetime, ClockFormat::TwentyFourHour);

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

        let clock = ClockState::from_datetime(datetime, ClockFormat::TwelveHour);

        assert_eq!(clock.primary_text(ClockStyle::Standard), "03:05");
        assert_eq!(clock.primary_text(ClockStyle::Stacked), "03");
        assert_eq!(clock.secondary_text(ClockStyle::Stacked), Some("05"));
        assert_eq!(clock.meridiem_text(), Some("PM"));
        assert_eq!(clock.date_text(), "Tuesday, March 24");
    }

    #[test]
    fn formats_shanghai_as_12_am() {
        let datetime = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::March, 24).expect("date"),
            Time::from_hms(0, 5, 0).expect("time"),
        )
        .assume_offset(UtcOffset::UTC);

        let clock = ClockState::from_datetime(datetime, ClockFormat::TwelveHour);

        assert_eq!(clock.primary_text(ClockStyle::Standard), "12:05");
        assert_eq!(clock.meridiem_text(), Some("AM"));
    }
}
