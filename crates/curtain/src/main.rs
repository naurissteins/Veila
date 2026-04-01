use std::fmt;

use time::{OffsetDateTime, UtcOffset};
use tracing_subscriber::fmt::time::FormatTime;

struct ShortLocalTime;

impl FormatTime for ShortLocalTime {
    fn format_time(&self, writer: &mut tracing_subscriber::fmt::format::Writer<'_>) -> fmt::Result {
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

fn main() -> anyhow::Result<()> {
    let options = veila_curtain::CurtainOptions::parse_args(std::env::args())?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_timer(ShortLocalTime)
        .init();

    if !options.help {
        tracing::info!(
            component = veila_curtain::component_name(),
            "starting curtain"
        );
    }

    veila_curtain::run(options)
}
