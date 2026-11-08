use crate::{background::BackgroundEvent, state::CurtainApp};
use calloop::channel::Sender;
use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_renderer::text::{FontLoader, FontRequest, PreparedFonts, install_fonts};

#[derive(Default)]
pub(crate) struct DeferredFonts {
    loader: Option<FontLoader>,
    generation: u64,
}

impl DeferredFonts {
    fn request(&mut self, requests: Vec<FontRequest>, sender: Sender<BackgroundEvent>) {
        self.generation = self.generation.wrapping_add(1);
        if self.loader.is_none() {
            match FontLoader::new(move |generation, fonts| {
                let _ = sender.send(BackgroundEvent::FontsReady {
                    generation,
                    fonts: Box::new(fonts),
                });
            }) {
                Ok(loader) => self.loader = Some(loader),
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

    pub(crate) fn accept_fonts(
        &mut self,
        generation: u64,
        fonts: PreparedFonts,
        queue: &QueueHandle<Self>,
    ) {
        if generation != self.fonts.generation || !install_fonts(fonts) {
            return;
        }
        self.ui_shell.invalidate_fonts();
        tracing::info!(
            generation,
            font_key = veila_renderer::text::font_cache_key(),
            "installed deferred fonts"
        );
        for surface in &mut self.lock_surfaces {
            surface.scene_base = None;
            surface.scene_base_revision = 0;
        }
        self.render_all_surfaces(queue);
    }
}
