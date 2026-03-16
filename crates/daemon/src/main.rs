#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!(
        component = kwylock_daemon::component_name(),
        "starting daemon"
    );

    let options = kwylock_daemon::DaemonOptions::parse_args(std::env::args())?;
    kwylock_daemon::run(options).await
}
