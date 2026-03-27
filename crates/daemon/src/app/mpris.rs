use std::{collections::HashMap, path::PathBuf, time::Duration};

use anyhow::Result;
use time::OffsetDateTime;
use tokio::sync::watch;
use veila_common::NowPlayingSnapshot;
use zbus::{Connection, Proxy, fdo::DBusProxy, zvariant::OwnedValue};

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const MPRIS_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

#[derive(Clone)]
pub(super) struct NowPlayingHandle {
    snapshot_rx: watch::Receiver<Option<NowPlayingSnapshot>>,
}

impl NowPlayingHandle {
    pub(super) fn spawn() -> Self {
        let (snapshot_tx, snapshot_rx) = watch::channel(None);

        tokio::spawn(async move {
            run_now_playing_service(snapshot_tx).await;
        });

        Self { snapshot_rx }
    }

    pub(super) fn current_snapshot(&self) -> Option<NowPlayingSnapshot> {
        self.snapshot_rx.borrow().clone()
    }
}

async fn run_now_playing_service(snapshot_tx: watch::Sender<Option<NowPlayingSnapshot>>) {
    loop {
        snapshot_tx.send_replace(fetch_snapshot_async().await);
        tokio::time::sleep(REFRESH_INTERVAL).await;
    }
}

async fn fetch_snapshot_async() -> Option<NowPlayingSnapshot> {
    match fetch_snapshot().await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::debug!("mpris refresh failed: {error:#}");
            None
        }
    }
}

async fn fetch_snapshot() -> Result<Option<NowPlayingSnapshot>> {
    let connection = Connection::session().await?;
    let dbus = DBusProxy::new(&connection).await?;
    let names = dbus.list_names().await?;
    let mut best = None;

    for name in names {
        let name = name.to_string();
        if !name.starts_with(MPRIS_PREFIX) {
            continue;
        }

        let Some(candidate) = player_snapshot(&connection, &name).await? else {
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
) -> Result<Option<(u8, NowPlayingSnapshot)>> {
    let proxy = Proxy::new(connection, bus_name, MPRIS_PATH, MPRIS_INTERFACE).await?;
    let playback_status: String = proxy.get_property("PlaybackStatus").await?;
    let Some(rank) = playback_rank(&playback_status) else {
        return Ok(None);
    };
    let metadata: HashMap<String, OwnedValue> = proxy.get_property("Metadata").await?;
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

fn playback_rank(status: &str) -> Option<u8> {
    match status {
        "Playing" => Some(2),
        "Paused" => Some(1),
        _ => None,
    }
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
