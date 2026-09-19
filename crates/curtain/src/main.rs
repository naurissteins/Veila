use std::fmt;

use anyhow::Context;
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
    nix::sys::prctl::set_dumpable(false).context("failed to disable curtain core dumps")?;
    let options = veila_curtain::CurtainOptions::parse_args(std::env::args())?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
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
