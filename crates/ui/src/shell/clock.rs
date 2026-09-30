use time::{Month, OffsetDateTime, Weekday};
use veila_common::{ClockFormat, ClockStyle, DateFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClockState {
    minute_key: i64,
    format: ClockFormat,
    date_format: DateFormat,
    time_text: String,
    hour_text: String,
    minute_text: String,
    meridiem_text: Option<String>,
    date_text: String,
}

impl ClockState {
    pub(super) fn current(format: ClockFormat, date_format: DateFormat) -> Self {
        Self::from_datetime(local_now(), format, date_format)
    }

    pub(super) fn refresh(&mut self) -> bool {
        self.refresh_at(local_now())
    }

    fn refresh_at(&mut self, datetime: OffsetDateTime) -> bool {
        if self.minute_key == local_minute_key(datetime) {
            return false;
        }

        *self = Self::from_datetime(datetime, self.format, self.date_format);
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

    fn from_datetime(
        datetime: OffsetDateTime,
        format: ClockFormat,
        date_format: DateFormat,
    ) -> Self {
        let (time_text, hour_text, minute_text, meridiem_text) = format_time(datetime, format);

        Self {
            minute_key: local_minute_key(datetime),
            format,
            date_format,
            time_text,
            hour_text,
            minute_text,
            meridiem_text,
            date_text: format_date(datetime, date_format),
        }
    }
}

impl super::ShellState {
    pub fn set_preview_time(&mut self, datetime: OffsetDateTime) {
        self.clock =
            ClockState::from_datetime(datetime, self.theme.clock_format, self.theme.date_format);
    }
}

fn local_minute_key(datetime: OffsetDateTime) -> i64 {
    // Include the offset so timezone changes cannot hide a changed clock or date.
    (datetime.unix_timestamp() + i64::from(datetime.offset().whole_seconds())).div_euclid(60)
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

fn format_date(datetime: OffsetDateTime, format: DateFormat) -> String {
    match format {
        DateFormat::Long => format!(
            "{}, {} {}",
            weekday_name(datetime.weekday()),
            month_name(datetime.month()),
            datetime.day()
        ),
        DateFormat::Iso => format!(
            "{:04}-{:02}-{:02}",
            datetime.year(),
            u8::from(datetime.month()),
            datetime.day()
        ),
        DateFormat::DayMonthYearDots => format!(
            "{:02}.{:02}.{:04}",
            datetime.day(),
            u8::from(datetime.month()),
            datetime.year()
        ),
        DateFormat::YearMonthDayDots => format!(
            "{:04}.{:02}.{:02}",
            datetime.year(),
            u8::from(datetime.month()),
            datetime.day()
        ),
        DateFormat::MonthDayYearSlash => format!(
            "{:02}/{:02}/{:04}",
            u8::from(datetime.month()),
            datetime.day(),
            datetime.year()
        ),
        DateFormat::DayMonthYearSlash => format!(
            "{:02}/{:02}/{:04}",
            datetime.day(),
            u8::from(datetime.month()),
            datetime.year()
        ),
        DateFormat::Short => format!(
            "{}, {} {}",
            short_weekday_name(datetime.weekday()),
            month_name(datetime.month()),
            datetime.day()
        ),
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

fn short_weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
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
mod tests;
