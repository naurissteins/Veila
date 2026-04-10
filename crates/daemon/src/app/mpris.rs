use std::{collections::HashMap, path::PathBuf, time::Duration};

use anyhow::Result;
use time::OffsetDateTime;
use tokio::sync::watch;
use veila_common::{NowPlayingConfig, NowPlayingSnapshot};
use zbus::{Connection, Proxy, fdo::DBusProxy, zvariant::OwnedValue};

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const MPRIS_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

#[derive(Clone)]
pub(super) struct NowPlayingHandle {
    config_tx: watch::Sender<NowPlayingConfig>,
    snapshot_rx: watch::Receiver<Option<NowPlayingSnapshot>>,
}

impl NowPlayingHandle {
    pub(super) fn spawn(config: &NowPlayingConfig) -> Self {
        let (config_tx, config_rx) = watch::channel(config.clone());
        let (snapshot_tx, snapshot_rx) = watch::channel(None);

        tokio::spawn(async move {
            run_now_playing_service(config_rx, snapshot_tx).await;
        });

        Self {
            config_tx,
            snapshot_rx,
        }
    }

    pub(super) fn current_snapshot(&self) -> Option<NowPlayingSnapshot> {
        self.snapshot_rx.borrow().clone()
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<Option<NowPlayingSnapshot>> {
        self.snapshot_rx.clone()
    }

    pub(super) fn update_config(&self, config: &NowPlayingConfig) {
        let _ = self.config_tx.send(config.clone());
    }
}

async fn run_now_playing_service(
    mut config_rx: watch::Receiver<NowPlayingConfig>,
    snapshot_tx: watch::Sender<Option<NowPlayingSnapshot>>,
) {
    let mut last_snapshot = None;
    let mut config = config_rx.borrow().clone();

    loop {
        let next_snapshot = fetch_snapshot_async(config.clone()).await;
        if !same_track_snapshot(last_snapshot.as_ref(), next_snapshot.as_ref()) {
            last_snapshot = next_snapshot.clone();
            snapshot_tx.send_replace(next_snapshot);
        }

        let refresh = tokio::time::sleep(REFRESH_INTERVAL);
        tokio::pin!(refresh);

        tokio::select! {
            _ = &mut refresh => {}
            changed = config_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                config = config_rx.borrow().clone();
            }
        }
    }
}

async fn fetch_snapshot_async(config: NowPlayingConfig) -> Option<NowPlayingSnapshot> {
    match fetch_snapshot(&config).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::debug!("mpris refresh failed: {error:#}");
            None
        }
    }
}

async fn fetch_snapshot(config: &NowPlayingConfig) -> Result<Option<NowPlayingSnapshot>> {
    let connection = Connection::session().await?;
    let dbus = DBusProxy::new(&connection).await?;
    let names = dbus.list_names().await?;
    let mut best = None;

    for name in names {
        let name = name.to_string();
        if !name.starts_with(MPRIS_PREFIX) {
            continue;
        }

        let Some(candidate) = player_snapshot(&connection, &name, config).await? else {
            continue;
        };

        let replace = best
            .as_ref()
            .is_none_or(|(best_rank, _)| candidate.0 > *best_rank);
        if replace {
            best = Some(candidate);
        }
    }

    Ok(best.map(|(_, snapshot)| snapshot))
}

async fn player_snapshot(
    connection: &Connection,
    bus_name: &str,
    config: &NowPlayingConfig,
) -> Result<Option<(u8, NowPlayingSnapshot)>> {
    let root_proxy = Proxy::new(connection, bus_name, MPRIS_PATH, "org.mpris.MediaPlayer2").await?;
    let player_proxy = Proxy::new(connection, bus_name, MPRIS_PATH, MPRIS_INTERFACE).await?;
    let player = PlayerDescriptor {
        bus_name,
        identity: property_string(&root_proxy, "Identity").await?,
        desktop_entry: property_string(&root_proxy, "DesktopEntry").await?,
    };

    if !player_is_included(&player, &config.include_players) {
        tracing::debug!(
            bus_name = player.bus_name,
            identity = player.identity.as_deref().unwrap_or("none"),
            desktop_entry = player.desktop_entry.as_deref().unwrap_or("none"),
            "skipping mpris player because it is not in the include list"
        );
        return Ok(None);
    }

    if player_is_excluded(&player, &config.exclude_players) {
        tracing::debug!(
            bus_name = player.bus_name,
            identity = player.identity.as_deref().unwrap_or("none"),
            desktop_entry = player.desktop_entry.as_deref().unwrap_or("none"),
            "skipping excluded mpris player"
        );
        return Ok(None);
    }

    let playback_status: String = player_proxy.get_property("PlaybackStatus").await?;
    let Some(rank) = playback_rank(&playback_status) else {
        return Ok(None);
    };
    let metadata: HashMap<String, OwnedValue> = player_proxy.get_property("Metadata").await?;
    let Some(title) = metadata_string(&metadata, "xesam:title") else {
        return Ok(None);
    };

    let snapshot = NowPlayingSnapshot {
        title,
        artist: metadata_string_list_first(&metadata, "xesam:artist"),
        artwork_path: metadata_string(&metadata, "mpris:artUrl").and_then(resolve_artwork_path),
        fetched_at_unix: OffsetDateTime::now_utc().unix_timestamp(),
    };

    Ok(Some((rank, snapshot)))
}

struct PlayerDescriptor<'a> {
    bus_name: &'a str,
    identity: Option<String>,
    desktop_entry: Option<String>,
}

fn playback_rank(status: &str) -> Option<u8> {
    match status {
        "Playing" => Some(2),
        "Paused" => Some(1),
        _ => None,
    }
}

async fn property_string(proxy: &Proxy<'_>, property: &str) -> Result<Option<String>> {
    let value: String = proxy.get_property(property).await?;
    Ok(normalize_string(value))
}

fn metadata_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = metadata.get(key)?.clone();
    normalize_string(String::try_from(value).ok()?)
}

fn metadata_string_list_first(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = metadata.get(key)?.clone();
    let values = Vec::<String>::try_from(value).ok()?;
    values.into_iter().find_map(normalize_string)
}

fn normalize_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn player_is_included(player: &PlayerDescriptor<'_>, include_players: &[String]) -> bool {
    include_players.is_empty() || player_matches_any_filter(player, include_players)
}

fn player_is_excluded(player: &PlayerDescriptor<'_>, exclude_players: &[String]) -> bool {
    player_matches_any_filter(player, exclude_players)
}

fn player_matches_any_filter(player: &PlayerDescriptor<'_>, filters: &[String]) -> bool {
    let Some(bus_suffix) = player.bus_name.strip_prefix(MPRIS_PREFIX) else {
        return false;
    };
    let bus_name = normalize_filter_value(bus_suffix);
    let bus_base = normalize_filter_value(bus_suffix.split('.').next().unwrap_or(bus_suffix));
    let identity = player.identity.as_deref().map(normalize_filter_value);
    let desktop_entry = player.desktop_entry.as_deref().map(normalize_filter_value);

    filters
        .iter()
        .filter_map(|entry| {
            let normalized = normalize_filter_value(entry);
            (!normalized.is_empty()).then_some(normalized)
        })
        .any(|entry| {
            identity.as_deref() == Some(entry.as_str())
                || desktop_entry.as_deref() == Some(entry.as_str())
                || bus_base == entry
                || bus_name == entry
                || bus_name.starts_with(&format!("{entry}."))
        })
}

fn normalize_filter_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn resolve_artwork_path(value: String) -> Option<PathBuf> {
    if let Some(path) = value.strip_prefix("file://localhost") {
        return normalize_path(path);
    }

    if let Some(path) = value.strip_prefix("file://") {
        return normalize_path(path);
    }

    normalize_path(&value)
}

fn normalize_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') {
        return None;
    }

    let path = PathBuf::from(trimmed);
    path.is_file().then_some(path)
}

fn same_track_snapshot(
    left: Option<&NowPlayingSnapshot>,
    right: Option<&NowPlayingSnapshot>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.title == right.title
                && left.artist == right.artist
                && left.artwork_path == right.artwork_path
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayerDescriptor, normalize_filter_value, player_is_excluded, player_is_included};

    #[test]
    fn excludes_players_by_identity_case_insensitively() {
        let player = PlayerDescriptor {
            bus_name: "org.mpris.MediaPlayer2.firefox",
            identity: Some(String::from("Firefox")),
            desktop_entry: Some(String::from("firefox")),
        };

        assert!(player_is_excluded(&player, &[String::from("firefox")]));
    }

    #[test]
    fn includes_all_players_when_include_list_is_empty() {
        let player = PlayerDescriptor {
            bus_name: "org.mpris.MediaPlayer2.firefox",
            identity: Some(String::from("Firefox")),
            desktop_entry: Some(String::from("firefox")),
        };

        assert!(player_is_included(&player, &[]));
    }

    #[test]
    fn includes_matching_players_by_identity_case_insensitively() {
        let player = PlayerDescriptor {
            bus_name: "org.mpris.MediaPlayer2.spotify",
            identity: Some(String::from("Spotify")),
            desktop_entry: Some(String::from("spotify")),
        };

        assert!(player_is_included(&player, &[String::from("spotify")]));
        assert!(!player_is_included(&player, &[String::from("firefox")]));
    }

    #[test]
    fn excludes_players_by_bus_name_base_for_instance_suffixes() {
        let player = PlayerDescriptor {
            bus_name: "org.mpris.MediaPlayer2.chromium.instance458",
            identity: None,
            desktop_entry: None,
        };

        assert!(player_is_excluded(&player, &[String::from("Chromium")]));
    }

    #[test]
    fn ignores_empty_filter_entries() {
        let player = PlayerDescriptor {
            bus_name: "org.mpris.MediaPlayer2.spotify",
            identity: Some(String::from("Spotify")),
            desktop_entry: Some(String::from("spotify")),
        };

        assert!(!player_is_excluded(
            &player,
            &[String::from(" "), String::from("")],
        ));
        assert!(!player_is_included(
            &player,
            &[String::from(" "), String::from("")],
        ));
        assert_eq!(normalize_filter_value(" Firefox "), "firefox");
    }

    #[test]
    fn exclude_filters_override_include_filters() {
        let player = PlayerDescriptor {
            bus_name: "org.mpris.MediaPlayer2.firefox",
            identity: Some(String::from("Firefox")),
            desktop_entry: Some(String::from("firefox")),
        };

        assert!(player_is_included(&player, &[String::from("Firefox")]));
        assert!(player_is_excluded(&player, &[String::from("Firefox")]));
    }
}
