use std::{
    io,
    path::PathBuf,
    sync::{Arc, OnceLock},
    thread,
    time::{Duration, Instant},
};

use calloop::channel::Sender;
use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_common::NowPlayingSnapshot;
use veila_renderer::cover::CoverArtAsset;
use veila_ui::{ShellState, WidgetKind};

use crate::state::{CurtainApp, elapsed_us};

use super::BackgroundEvent;

const ARTWORK_RETRY_INTERVAL: Duration = Duration::from_secs(2);

struct DecodedArtwork {
    asset: CoverArtAsset,
    elapsed_ms: u128,
}

pub(crate) struct ArtworkLoad {
    path: PathBuf,
    snapshot: Option<NowPlayingSnapshot>,
    decoded: Arc<OnceLock<DecodedArtwork>>,
    applied_during_startup: bool,
}

impl ArtworkLoad {
    fn spawn(
        path: PathBuf,
        snapshot: Option<NowPlayingSnapshot>,
        max_dimension: u32,
        sender: Sender<BackgroundEvent>,
    ) -> io::Result<Self> {
        let decoded = Arc::new(OnceLock::new());
        let worker_result = Arc::clone(&decoded);
        let worker_path = path.clone();
        thread::Builder::new()
            .name("veila-artwork".into())
            .spawn(move || {
                let started_at = Instant::now();
                match CoverArtAsset::load(&worker_path, max_dimension) {
                    Ok(asset) => {
                        let _ = worker_result.set(DecodedArtwork {
                            asset,
                            elapsed_ms: started_at.elapsed().as_millis(),
                        });
                    }
                    Err(error) => {
                        tracing::debug!(path = %worker_path.display(), "failed to load now playing artwork: {error}");
                    }
                }
                let _ = sender.send(BackgroundEvent::ArtworkReady);
            })?;

        Ok(Self {
            path,
            snapshot,
            decoded,
            applied_during_startup: false,
        })
    }

    pub(crate) fn apply_for_first_frame(
        &mut self,
        snapshot: Option<&NowPlayingSnapshot>,
        shell: &mut ShellState,
    ) -> bool {
        let Some(artwork) = self.current_artwork(snapshot, shell) else {
            return false;
        };
        self.applied_during_startup |=
            shell.set_now_playing_artwork(&self.path, artwork.asset.clone());
        self.applied_during_startup
    }

    fn current_artwork(
        &self,
        snapshot: Option<&NowPlayingSnapshot>,
        shell: &ShellState,
    ) -> Option<&DecodedArtwork> {
        if self.snapshot.as_ref() != snapshot || shell.emergency_active() {
            return None;
        }
        self.decoded.get()
    }
}

impl CurtainApp {
    pub(crate) fn maybe_start_artwork_load(&mut self) {
        if self.ui_shell.emergency_active()
            || !self.session_locked
            || self.artwork_in_flight.is_some()
        {
            return;
        }
        let Some(path) = self
            .ui_shell
            .pending_now_playing_artwork_path()
            .map(std::path::Path::to_path_buf)
        else {
            return;
        };
        if self
            .artwork_last_attempt
            .as_ref()
            .is_some_and(|(previous, attempted_at)| {
                previous == &path && attempted_at.elapsed() < ARTWORK_RETRY_INTERVAL
            })
        {
            return;
        }
        let max_dimension = self
            .lock_surfaces
            .iter()
            .filter_map(|surface| surface.size)
            .map(|size| {
                self.ui_shell
                    .now_playing_artwork_decode_size_at_scale(size.buffer, size.render_scale)
            })
            .max()
            .unwrap_or(160);
        self.artwork_last_attempt = Some((path.clone(), Instant::now()));
        match ArtworkLoad::spawn(
            path.clone(),
            self.now_playing_snapshot.clone(),
            max_dimension,
            self.background_sender.clone(),
        ) {
            Ok(load) => self.artwork_in_flight = Some(load),
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "failed to start artwork worker");
            }
        }
    }

    pub(super) fn finish_artwork_load(&mut self, queue: &QueueHandle<Self>) {
        let Some(load) = self.artwork_in_flight.take() else {
            return;
        };
        if let Some(artwork) =
            load.current_artwork(self.now_playing_snapshot.as_ref(), &self.ui_shell)
            && (self
                .ui_shell
                .set_now_playing_artwork(&load.path, artwork.asset.clone())
                || load.applied_during_startup)
        {
            tracing::debug!(
                elapsed_ms = artwork.elapsed_ms,
                startup_elapsed_us = elapsed_us(self.startup_started_at),
                "loaded deferred now playing artwork"
            );
            // The wake also updates outputs that committed before the decode finished.
            self.render_widget_surfaces(WidgetKind::Media, queue);
        }
        self.maybe_start_artwork_load();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_frame_uses_ready_artwork_only_for_the_current_track() {
        let path = std::env::temp_dir().join(format!(
            "veila-first-frame-cover-{}.png",
            std::process::id()
        ));
        let png = [
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0,
            5, 0, 1, 13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        std::fs::write(&path, png).unwrap();
        let asset = CoverArtAsset::load(&path, 32).unwrap();
        std::fs::remove_file(&path).unwrap();

        let config = veila_common::AppConfig::from_toml_str(
            "[visuals.now_playing.artwork]\nenabled = true\nhalign = \"left\"\nvalign = \"top\"",
        )
        .unwrap();
        let mut shell =
            ShellState::new(veila_ui::ShellTheme::from_config(&config), None, None, true);
        let track = NowPlayingSnapshot {
            title: "Current track".into(),
            artist: None,
            artwork_path: Some(path.clone()),
            fetched_at_unix: 1,
        };
        let mut load = ArtworkLoad {
            path,
            snapshot: Some(track.clone()),
            decoded: Arc::new(OnceLock::new()),
            applied_during_startup: false,
        };
        shell.set_initial_now_playing_snapshot(track.clone());

        assert!(!load.apply_for_first_frame(Some(&track), &mut shell));
        assert!(
            load.decoded
                .set(DecodedArtwork {
                    asset,
                    elapsed_ms: 1
                })
                .is_ok()
        );
        let previous_track = NowPlayingSnapshot {
            title: "Previous track".into(),
            ..track.clone()
        };
        assert!(!load.apply_for_first_frame(Some(&previous_track), &mut shell));
        assert!(shell.pending_now_playing_artwork_path().is_some());

        assert!(load.apply_for_first_frame(Some(&track), &mut shell));
        assert!(shell.pending_now_playing_artwork_path().is_none());
        assert!(load.apply_for_first_frame(Some(&track), &mut shell));
    }
}
