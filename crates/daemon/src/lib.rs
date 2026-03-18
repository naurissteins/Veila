#![forbid(unsafe_code)]

//! Daemon entrypoints for Kwylock lock orchestration.

mod adapters;
mod app;
mod domain;

use std::path::PathBuf;

use anyhow::{Result, bail};

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "kwylockd"
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonOptions {
    pub config_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub lock_now: bool,
}

impl DaemonOptions {
    pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();

        for arg in args.into_iter().skip(1) {
            if let Some(path) = arg.strip_prefix("--config=") {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(session_id) = arg.strip_prefix("--session-id=") {
                options.session_id = Some(session_id.to_string());
                continue;
            }

            if arg == "--lock-now" {
                options.lock_now = true;
                continue;
            }

            bail!("unknown daemon argument: {arg}");
        }

        Ok(options)
    }
}

/// Starts the daemon runtime.
pub async fn run(options: DaemonOptions) -> anyhow::Result<()> {
    app::run(options).await
}

#[cfg(test)]
mod tests {
    use super::DaemonOptions;

    #[test]
    fn parses_config_argument() {
        let options = DaemonOptions::parse_args([
            "kwylockd".to_string(),
            "--config=/tmp/kwylock.toml".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.config_path.as_deref(),
            Some(std::path::Path::new("/tmp/kwylock.toml"))
        );
    }

    #[test]
    fn parses_session_id_argument() {
        let options =
            DaemonOptions::parse_args(["kwylockd".to_string(), "--session-id=c2".to_string()])
                .expect("arguments should parse");

        assert_eq!(options.session_id.as_deref(), Some("c2"));
    }

    #[test]
    fn parses_lock_now_argument() {
        let options = DaemonOptions::parse_args(["kwylockd".to_string(), "--lock-now".to_string()])
            .expect("arguments should parse");

        assert!(options.lock_now);
    }
}
