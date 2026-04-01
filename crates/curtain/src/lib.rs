#![forbid(unsafe_code)]

//! Secure session-lock curtain for Veila.

mod app;
mod background;
mod ipc;
mod preview;
mod reload;
mod state;
mod wayland;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use veila_common::{BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot, ipc::decode_message};

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "veila-curtain"
}

/// Command-line options for the curtain process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurtainOptions {
    pub help: bool,
    pub notify_socket: Option<PathBuf>,
    pub daemon_socket: Option<PathBuf>,
    pub control_socket: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub preview_png: Option<PathBuf>,
    pub preview_size: Option<veila_renderer::FrameSize>,
    pub preview_artwork: Option<PathBuf>,
    pub preview_title: Option<String>,
    pub preview_artist: Option<String>,
    pub weather_snapshot: Option<WeatherSnapshot>,
    pub battery_snapshot: Option<BatterySnapshot>,
    pub now_playing_snapshot: Option<NowPlayingSnapshot>,
}

impl CurtainOptions {
    /// Parses curtain options from an iterator of process arguments.
    pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();

        for arg in args.into_iter().skip(1) {
            if arg == "--help" || arg == "-h" {
                options.help = true;
                continue;
            }

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

            if let Some(path) = arg.strip_prefix("--preview-png=") {
                options.preview_png = Some(PathBuf::from(path));
                continue;
            }

            if let Some(size) = arg.strip_prefix("--preview-size=") {
                options.preview_size =
                    Some(parse_preview_size(size).context("failed to parse preview size")?);
                continue;
            }

            if let Some(path) = arg.strip_prefix("--preview-artwork=") {
                options.preview_artwork = Some(PathBuf::from(path));
                continue;
            }

            if let Some(title) = arg.strip_prefix("--preview-title=") {
                options.preview_title = Some(title.to_string());
                continue;
            }

            if let Some(artist) = arg.strip_prefix("--preview-artist=") {
                options.preview_artist = Some(artist.to_string());
                continue;
            }

            if let Some(snapshot) = arg.strip_prefix("--weather-snapshot=") {
                options.weather_snapshot =
                    Some(decode_message(snapshot).context("failed to decode weather snapshot")?);
                continue;
            }

            if let Some(snapshot) = arg.strip_prefix("--battery-snapshot=") {
                options.battery_snapshot =
                    Some(decode_message(snapshot).context("failed to decode battery snapshot")?);
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
    if options.help {
        print_help();
        return Ok(());
    }

    app::run(options)
}

fn print_help() {
    println!(
        "\
Veila secure curtain and preview CLI

Usage:
  {name} [options]

General:
  -h, --help                         Show this help text
      --config=<path>                Use a specific config file
      --notify-socket=<path>         Notify socket for curtain readiness
      --daemon-socket=<path>         Daemon auth IPC socket
      --control-socket=<path>        Curtain live-control IPC socket

Preview mode:
      --preview-png=<path>           Render the scene to a PNG instead of locking
      --preview-size=<width>x<height>  Output size for preview rendering
      --preview-artwork=<path>       Override now playing artwork for preview
      --preview-title=<text>         Override now playing title for preview
      --preview-artist=<text>        Override now playing artist for preview

Daemon snapshot overrides:
      --weather-snapshot=<payload>   Inject a weather snapshot
      --battery-snapshot=<payload>   Inject a battery snapshot
      --now-playing-snapshot=<payload>  Inject a now playing snapshot

Notes:
  If no preview option is given, {name} starts the secure session-lock curtain.
  --preview-png renders directly to a PNG without taking a real lock.
",
        name = component_name()
    );
}

fn parse_preview_size(input: &str) -> Result<veila_renderer::FrameSize> {
    let (width, height) = input
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("preview size must use WIDTHxHEIGHT"))?;
    let width = width.parse::<u32>().context("invalid preview width")?;
    let height = height.parse::<u32>().context("invalid preview height")?;

    if width == 0 || height == 0 {
        bail!("preview size must be non-zero");
    }

    Ok(veila_renderer::FrameSize::new(width, height))
}

#[cfg(test)]
mod tests {
    use veila_common::{BatterySnapshot, NowPlayingSnapshot, ipc::encode_message};

    use super::CurtainOptions;

    #[test]
    fn parses_notify_socket_argument() {
        let options = CurtainOptions::parse_args([
            "veila-curtain".to_string(),
            "--notify-socket=/tmp/veila.sock".to_string(),
            "--daemon-socket=/tmp/veila-auth.sock".to_string(),
            "--control-socket=/tmp/veila-control.sock".to_string(),
            "--config=/tmp/veila.toml".to_string(),
            "--preview-png=/tmp/veila-preview.png".to_string(),
            "--preview-size=1920x1080".to_string(),
            "--preview-artwork=/tmp/cover.png".to_string(),
            "--preview-title=After Dark".to_string(),
            "--preview-artist=Mr.Kitty".to_string(),
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
        assert_eq!(
            options.preview_png.as_deref(),
            Some(std::path::Path::new("/tmp/veila-preview.png"))
        );
        assert_eq!(
            options.preview_size,
            Some(veila_renderer::FrameSize::new(1920, 1080))
        );
        assert_eq!(
            options.preview_artwork.as_deref(),
            Some(std::path::Path::new("/tmp/cover.png"))
        );
        assert_eq!(options.preview_title.as_deref(), Some("After Dark"));
        assert_eq!(options.preview_artist.as_deref(), Some("Mr.Kitty"));
    }

    #[test]
    fn parses_help_arguments() {
        let long = CurtainOptions::parse_args(["veila-curtain".to_string(), "--help".to_string()])
            .expect("arguments should parse");
        let short = CurtainOptions::parse_args(["veila-curtain".to_string(), "-h".to_string()])
            .expect("arguments should parse");

        assert!(long.help);
        assert!(short.help);
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

    #[test]
    fn parses_battery_snapshot_argument() {
        let encoded = encode_message(&BatterySnapshot {
            percent: 84,
            charging: true,
        })
        .expect("snapshot");
        let options = CurtainOptions::parse_args([
            String::from("veila-curtain"),
            format!("--battery-snapshot={encoded}"),
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.battery_snapshot,
            Some(BatterySnapshot {
                percent: 84,
                charging: true,
            })
        );
    }
}
