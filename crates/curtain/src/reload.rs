use anyhow::Context;
use kwylock_common::AppConfig;
use kwylock_renderer::background::BackgroundAsset;
use kwylock_ui::ShellTheme;
use smithay_client_toolkit::reexports::client::QueueHandle;

use crate::state::CurtainApp;

impl CurtainApp {
    pub(crate) fn reload_config(&mut self, queue_handle: &QueueHandle<Self>) {
        let loaded_config = match AppConfig::load(self.config_path.as_deref()) {
            Ok(config) => config,
            Err(error) => {
                tracing::warn!("failed to reload curtain config: {error:#}");
                return;
            }
        };
        let config = loaded_config.config;
        let theme = ShellTheme::from_config(&config);
        let background_asset = match BackgroundAsset::load(None, theme.background)
            .context("failed to prepare fallback background")
        {
            Ok(asset) => asset,
            Err(error) => {
                tracing::warn!("failed to reload curtain fallback background: {error:#}");
                return;
            }
        };
        let background_path = config.background.path.clone();

        self.background_color = theme.background;
        self.background_asset = background_asset;
        self.background_path = background_path.clone();
        self.lock_wait_timeout =
            std::time::Duration::from_secs(config.lock.acquire_timeout_seconds.max(1));
        self.ui_shell
            .apply_theme(theme, config.lock.user_hint.clone());
        self.background_render_started = false;
        for surface in &mut self.lock_surfaces {
            surface.background = None;
        }

        tracing::info!(
            config = loaded_config
                .path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "defaults".to_string()),
            background_image = background_path
                .as_deref()
                .map(|path| path.display().to_string()),
            "reloaded curtain config"
        );

        self.render_all_surfaces(queue_handle);
        self.maybe_start_background_render();
    }
}
