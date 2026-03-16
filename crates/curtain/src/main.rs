fn main() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    tracing::info!(
        component = kwylock_curtain::component_name(),
        "curtain stub started"
    );
}
