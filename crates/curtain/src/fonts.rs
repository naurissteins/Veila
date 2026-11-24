use crate::{background::BackgroundEvent, state::CurtainApp};
use calloop::channel::Sender;
use smithay_client_toolkit::reexports::client::QueueHandle;
use std::sync::mpsc::{self, Receiver};
use veila_renderer::text::{FontLoader, FontRequest, PreparedFonts, install_fonts};

#[derive(Default)]
pub(crate) struct DeferredFonts {
    loader: Option<FontLoader>,
    ready: Option<Receiver<(u64, PreparedFonts)>>,
    generation: u64,
}

impl DeferredFonts {
    fn request(&mut self, requests: Vec<FontRequest>, sender: Sender<BackgroundEvent>) {
        self.generation = self.generation.wrapping_add(1);
        if self.loader.is_none() {
            let (results, ready) = mpsc::channel();
            match FontLoader::new(move |generation, fonts| {
                if results.send((generation, fonts)).is_ok() {
                    let _ = sender.send(BackgroundEvent::FontsReady);
                }
            }) {
                Ok(loader) => {
                    self.loader = Some(loader);
                    self.ready = Some(ready);
                }
                Err(error) => {
                    tracing::warn!(%error, "optional font worker unavailable; using bundled fonts")
                }
            }
        }
        if let Some(loader) = &self.loader {
            loader.request(self.generation, requests);
        }
    }
}

impl CurtainApp {
    pub(crate) fn maybe_start_fonts(&mut self) {
        if self.session_locked && self.fonts.loader.is_none() {
            self.reload_fonts();
        }
    }

    pub(crate) fn reload_fonts(&mut self) {
        if self.session_locked {
            self.fonts.request(
                self.ui_shell.font_requests(),
                self.background_sender.clone(),
            );
        }
    }

    pub(crate) fn accept_fonts(&mut self, queue: &QueueHandle<Self>) {
        if self.install_ready_fonts(false) {
            self.render_all_surfaces(queue);
        }
    }

    pub(crate) fn apply_startup_fonts(&mut self) -> bool {
        if !self.session_locked
            || self.rich_scene_ready
            || self.ui_shell.emergency_active()
            || self
                .lock_surfaces
                .iter()
                .enumerate()
                .any(|(index, surface)| {
                    self.ui_visible_on_surface(index)
                        && surface
                            .shm_pool
                            .as_ref()
                            .is_some_and(|pool| pool.has_committed_frame())
                })
        {
            return false;
        }
        self.install_ready_fonts(true)
    }

    fn install_ready_fonts(&mut self, startup: bool) -> bool {
        let Some((generation, fonts)) = self.fonts.ready.as_ref().and_then(|ready| {
            ready
                .try_iter()
                .filter(|(generation, _)| *generation == self.fonts.generation)
                .last()
        }) else {
            return false;
        };
        if !install_fonts(fonts) {
            return false;
        }
        self.ui_shell.invalidate_fonts();
        tracing::info!(
            generation,
            font_key = veila_renderer::text::font_cache_key(),
            startup,
            startup_elapsed_us = crate::state::elapsed_us(self.startup_started_at),
            "installed deferred fonts"
        );
        for surface in &mut self.lock_surfaces {
            surface.scene_base = None;
            surface.scene_base_revision = 0;
        }
        true
    }
}
