use std::collections::HashMap;

use anyhow::Result;
use time::OffsetDateTime;
use veila_common::{NowPlayingConfig, NowPlayingSnapshot};
use zbus::{Connection, Proxy, fdo::DBusProxy, zvariant::OwnedValue};

use super::{
    LOG_TARGET, NowPlayingRefresh,
    metadata::{
        metadata_artwork_url, metadata_string, metadata_string_list_first, normalize_string,
        resolve_artwork_path,
    },
};

#[cfg(test)]
mod tests;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
pub(super) const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
pub(super) const MPRIS_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

pub(super) async fn fetch_snapshot(
    connection: &Connection,
    config: &NowPlayingConfig,
) -> Result<NowPlayingRefresh> {
    let dbus = DBusProxy::new(connection).await?;
    let names = dbus.list_names().await?;
    let mut best = None;

    for name in names {
        let name = name.to_string();
        if !name.starts_with(MPRIS_PREFIX) {
            continue;
        }

        let Some(candidate) = player_snapshot(connection, &name, config).await? else {
            continue;
        };

        // Equal ranks retain the first eligible player in the D-Bus name list.
        let replace = best
            .as_ref()
            .is_none_or(|best_candidate: &PlayerCandidate| candidate.rank > best_candidate.rank);
        if replace {
            best = Some(candidate);
        }
    }

    if let Some(candidate) = best {
        tracing::debug!(
            target: LOG_TARGET,
            bus_name = candidate.player.bus_name,
            identity = candidate.player.identity.as_deref().unwrap_or("none"),
            desktop_entry = candidate.player.desktop_entry.as_deref().unwrap_or("none"),
            rank = candidate.rank,
            title = candidate.snapshot.title,
            artist = candidate.snapshot.artist.as_deref().unwrap_or("none"),
            "selected mpris player for now playing widget"
        );
        return Ok(NowPlayingRefresh {
            playback_active: candidate.rank >= 2,
            snapshot: Some(candidate.snapshot),
        });
    }

    tracing::debug!(
        target: LOG_TARGET,
        "no eligible mpris player selected for now playing widget"
    );
    Ok(NowPlayingRefresh {
        snapshot: None,
        playback_active: false,
    })
}

async fn player_snapshot(
    connection: &Connection,
    bus_name: &str,
    config: &NowPlayingConfig,
) -> Result<Option<PlayerCandidate>> {
    let root_proxy = Proxy::new(connection, bus_name, MPRIS_PATH, "org.mpris.MediaPlayer2").await?;
    let player_proxy = Proxy::new(connection, bus_name, MPRIS_PATH, MPRIS_INTERFACE).await?;
    let player = PlayerDescriptor {
        bus_name: bus_name.to_string(),
        identity: property_string(&root_proxy, "Identity").await?,
        desktop_entry: optional_property_string(
            property_string(&root_proxy, "DesktopEntry").await,
            bus_name,
            "DesktopEntry",
        ),
    };

    if !player_is_included(&player, &config.include_players) {
        tracing::debug!(
            target: LOG_TARGET,
            bus_name = player.bus_name,
            identity = player.identity.as_deref().unwrap_or("none"),
            desktop_entry = player.desktop_entry.as_deref().unwrap_or("none"),
            "skipping mpris player because it is not in the include list"
        );
        return Ok(None);
    }

    if player_is_excluded(&player, &config.exclude_players) {
        tracing::debug!(
            target: LOG_TARGET,
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
        artwork_path: metadata_artwork_url(&metadata).and_then(resolve_artwork_path),
        fetched_at_unix: OffsetDateTime::now_utc().unix_timestamp(),
    };

    Ok(Some(PlayerCandidate {
        rank,
        player,
        snapshot,
    }))
}

struct PlayerCandidate {
    rank: u8,
    player: PlayerDescriptor,
    snapshot: NowPlayingSnapshot,
}

struct PlayerDescriptor {
    bus_name: String,
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

fn optional_property_string(
    result: Result<Option<String>>,
    bus_name: &str,
    property: &str,
) -> Option<String> {
    match result {
        Ok(value) => value,
        Err(error) => {
            tracing::debug!(
            target: LOG_TARGET,
                bus_name,
                property,
                "optional mpris property unavailable: {error:#}"
            );
            None
        }
    }
}

fn player_is_included(player: &PlayerDescriptor, include_players: &[String]) -> bool {
    include_players.is_empty() || player_matches_any_filter(player, include_players)
}

fn player_is_excluded(player: &PlayerDescriptor, exclude_players: &[String]) -> bool {
    player_matches_any_filter(player, exclude_players)
}

fn player_matches_any_filter(player: &PlayerDescriptor, filters: &[String]) -> bool {
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
