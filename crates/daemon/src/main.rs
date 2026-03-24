#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = veila_daemon::DaemonOptions::parse_args(std::env::args())?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
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
