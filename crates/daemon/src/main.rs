#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = kwylock_daemon::DaemonOptions::parse_args(std::env::args())?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    if !options.status && !options.health && !options.reload_config {
        tracing::info!(
            component = kwylock_daemon::component_name(),
            "starting daemon"
        );
    }

    kwylock_daemon::run(options).await
}
