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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = veila_daemon::DaemonOptions::parse_args(std::env::args())?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_timer(ShortLocalTime)
        .init();

    if !options.stop
        && !options.status
        && !options.health
        && !options.version
        && !options.reload_config
    {
        tracing::info!(
            component = veila_daemon::component_name(),
            "starting daemon"
        );
    }

    veila_daemon::run(options).await
}
