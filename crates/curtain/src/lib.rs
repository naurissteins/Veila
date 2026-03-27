#![forbid(unsafe_code)]

//! Secure session-lock curtain for Veila.

mod app;
mod background;
mod ipc;
mod reload;
mod state;
mod wayland;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use veila_common::{NowPlayingSnapshot, WeatherSnapshot, ipc::decode_message};

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "veila-curtain"
}

/// Command-line options for the curtain process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurtainOptions {
    pub notify_socket: Option<PathBuf>,
    pub daemon_socket: Option<PathBuf>,
    pub control_socket: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub weather_snapshot: Option<WeatherSnapshot>,
    pub now_playing_snapshot: Option<NowPlayingSnapshot>,
}

impl CurtainOptions {
    /// Parses curtain options from an iterator of process arguments.
    pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();

        for arg in args.into_iter().skip(1) {
            if let Some(path) = arg.strip_prefix("--notify-socket=") {
                options.notify_socket = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = arg.strip_prefix("--daemon-socket=") {
                options.daemon_socket = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = arg.strip_prefix("--control-socket=") {
                options.control_socket = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = arg.strip_prefix("--config=") {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(snapshot) = arg.strip_prefix("--weather-snapshot=") {
                options.weather_snapshot =
                    Some(decode_message(snapshot).context("failed to decode weather snapshot")?);
                continue;
            }

            if let Some(snapshot) = arg.strip_prefix("--now-playing-snapshot=") {
                options.now_playing_snapshot = Some(
                    decode_message(snapshot).context("failed to decode now playing snapshot")?,
                );
                continue;
            }

            bail!("unknown curtain argument: {arg}");
        }

        Ok(options)
    }
}

/// Starts the secure curtain process.
pub fn run(options: CurtainOptions) -> Result<()> {
    app::run(options)
}

#[cfg(test)]
mod tests {
    use veila_common::{NowPlayingSnapshot, ipc::encode_message};

    use super::CurtainOptions;

    #[test]
    fn parses_notify_socket_argument() {
        let options = CurtainOptions::parse_args([
            "veila-curtain".to_string(),
            "--notify-socket=/tmp/veila.sock".to_string(),
            "--daemon-socket=/tmp/veila-auth.sock".to_string(),
            "--control-socket=/tmp/veila-control.sock".to_string(),
            "--config=/tmp/veila.toml".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.notify_socket.as_deref(),
            Some(std::path::Path::new("/tmp/veila.sock"))
        );
        assert_eq!(
            options.daemon_socket.as_deref(),
            Some(std::path::Path::new("/tmp/veila-auth.sock"))
        );
        assert_eq!(
            options.control_socket.as_deref(),
            Some(std::path::Path::new("/tmp/veila-control.sock"))
        );
        assert_eq!(
            options.config_path.as_deref(),
            Some(std::path::Path::new("/tmp/veila.toml"))
        );
    }

    #[test]
    fn parses_now_playing_snapshot_argument() {
        let encoded = encode_message(&NowPlayingSnapshot {
            title: String::from("Track"),
            artist: Some(String::from("Artist")),
            artwork_path: None,
            fetched_at_unix: 7,
        })
        .expect("snapshot");
        let options = CurtainOptions::parse_args([
            String::from("veila-curtain"),
            format!("--now-playing-snapshot={encoded}"),
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.now_playing_snapshot,
            Some(NowPlayingSnapshot {
                title: String::from("Track"),
                artist: Some(String::from("Artist")),
                artwork_path: None,
                fetched_at_unix: 7,
            })
        );
    }
}
