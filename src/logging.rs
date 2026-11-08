use std::fmt;

use time::{OffsetDateTime, UtcOffset};
use tracing_subscriber::{EnvFilter, fmt::format::Writer, fmt::time::FormatTime};

struct ShortLocalTime;

impl FormatTime for ShortLocalTime {
    fn format_time(&self, writer: &mut Writer<'_>) -> fmt::Result {
        let now = OffsetDateTime::now_utc()
            .to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
        write!(
            writer,
            "{:02}:{:02}:{:02}",
            now.hour(),
            now.minute(),
            now.second()
        )
    }
}

pub(crate) fn init_stderr() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(env_filter())
        .with_timer(ShortLocalTime)
        .init();
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}
