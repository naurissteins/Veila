use std::time::Duration;

use tokio::sync::watch;
use veila_common::{NowPlayingConfig, NowPlayingSnapshot};

mod metadata;
mod query;
mod signals;
#[cfg(test)]
mod tests;

use signals::MprisClient;

const FALLBACK_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
// preserve existing module filters when query logs move into a child module.
const LOG_TARGET: &str = module_path!();

#[derive(Clone)]
pub(super) struct NowPlayingHandle {
    config_tx: watch::Sender<NowPlayingConfig>,
    snapshot_rx: watch::Receiver<Option<NowPlayingSnapshot>>,
    playback_active_rx: watch::Receiver<bool>,
}

impl NowPlayingHandle {
    pub(super) fn spawn(config: &NowPlayingConfig) -> Self {
        let (config_tx, config_rx) = watch::channel(config.clone());
        let (snapshot_tx, snapshot_rx) = watch::channel(None);
        let (playback_active_tx, playback_active_rx) = watch::channel(false);

        tokio::spawn(async move {
            run_now_playing_service(config_rx, snapshot_tx, playback_active_tx).await;
        });

        Self {
            config_tx,
            snapshot_rx,
            playback_active_rx,
        }
    }

    pub(super) fn current_snapshot(&self) -> Option<NowPlayingSnapshot> {
        self.snapshot_rx.borrow().clone()
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<Option<NowPlayingSnapshot>> {
        self.snapshot_rx.clone()
    }

    pub(super) fn currently_playing(&self) -> bool {
        *self.playback_active_rx.borrow()
    }

    pub(super) fn update_config(&self, config: &NowPlayingConfig) {
        let _ = self.config_tx.send(config.clone());
    }
}

async fn run_now_playing_service(
    mut config_rx: watch::Receiver<NowPlayingConfig>,
    snapshot_tx: watch::Sender<Option<NowPlayingSnapshot>>,
    playback_active_tx: watch::Sender<bool>,
) {
    let mut last_snapshot = None;
    let mut last_playback_active = false;
    let mut config = config_rx.borrow().clone();
    let mut client = None;

    loop {
        let refresh = fetch_refresh_state(&mut client, &config).await;
        if !same_track_snapshot(last_snapshot.as_ref(), refresh.snapshot.as_ref()) {
            last_snapshot = refresh.snapshot.clone();
            snapshot_tx.send_replace(refresh.snapshot);
        }
        if refresh.playback_active != last_playback_active {
            last_playback_active = refresh.playback_active;
            playback_active_tx.send_replace(refresh.playback_active);
        }

        let fallback_refresh = tokio::time::sleep(FALLBACK_REFRESH_INTERVAL);
        tokio::pin!(fallback_refresh);

        if let Some(active_client) = client.as_mut() {
            tokio::select! {
                _ = &mut fallback_refresh => {}
                signal = active_client.wait_for_change() => {
                    match signal {
                        Ok(reason) => tracing::debug!(?reason, "mpris refresh triggered by dbus signal"),
                        Err(error) => {
                            client = None;
                            tracing::debug!("mpris signal stream failed: {error:#}");
                        }
                    }
                }
                changed = config_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    config = config_rx.borrow().clone();
                }
            }
        } else {
            tokio::select! {
                _ = &mut fallback_refresh => {}
                changed = config_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    config = config_rx.borrow().clone();
                }
            }
        }
    }
}

struct NowPlayingRefresh {
    snapshot: Option<NowPlayingSnapshot>,
    playback_active: bool,
}

async fn fetch_refresh_state(
    client: &mut Option<MprisClient>,
    config: &NowPlayingConfig,
) -> NowPlayingRefresh {
    let refresh = match client {
        Some(client) => client.refresh(config).await,
        None => match MprisClient::connect().await {
            Ok(connected) => {
                let refresh = connected.refresh(config).await;
                *client = Some(connected);
                refresh
            }
            Err(error) => Err(error),
        },
    };

    match refresh {
        Ok(refresh) => refresh,
        Err(error) => {
            *client = None;
            tracing::debug!("mpris refresh failed: {error:#}");
            NowPlayingRefresh {
                snapshot: None,
                playback_active: false,
            }
        }
    }
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
