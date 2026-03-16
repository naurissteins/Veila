fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!(
        component = kwylock_curtain::component_name(),
        "starting curtain"
    );

    let options = kwylock_curtain::CurtainOptions::parse_args(std::env::args())?;
    kwylock_curtain::run(options)
}
