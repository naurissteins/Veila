#![forbid(unsafe_code)]

//! Daemon entrypoints for Kwylock lock orchestration.

mod adapters;
mod app;
mod domain;

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "kwylockd"
}

/// Starts the daemon runtime.
pub async fn run() -> anyhow::Result<()> {
    app::run().await
}
