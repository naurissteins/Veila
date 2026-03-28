use anyhow::Context;
use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_common::AppConfig;
use veila_renderer::background::BackgroundAsset;
use veila_ui::ShellTheme;

use crate::state::{CurtainApp, background_treatment};

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
        let background_asset = match BackgroundAsset::load(
            None,
            theme.background,
            background_treatment(&config.background),
        )
        .context("failed to prepare fallback background")
        {
            Ok(asset) => asset,
            Err(error) => {
                tracing::warn!("failed to reload curtain fallback background: {error:#}");
                return;
            }
        };
        let background_path = config.background.resolved_path();

        self.background_color = theme.background;
        self.background_asset = background_asset;
        self.background_treatment = background_treatment(&config.background);
        self.background_path = background_path.clone();
        self.lock_wait_timeout =
            std::time::Duration::from_secs(config.lock.acquire_timeout_seconds.max(1));
        self.ui_shell.apply_theme_with_username_and_weather(
            theme,
            config.lock.user_hint.clone(),
            config.lock.username.clone(),
            config.lock.avatar_path.clone(),
            config.lock.show_username,
            config.weather.normalized_location(),
            self.weather_snapshot.clone(),
            config.weather.unit,
            self.battery_snapshot.clone(),
            self.now_playing_snapshot.clone(),
        );
        self.background_render_started = false;
        for surface in &mut self.lock_surfaces {
            surface.background = None;
            surface.static_overlay = None;
            surface.static_overlay_revision = 0;
        }

        tracing::info!(
            config = loaded_config
                .path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "defaults".to_string()),
            background_mode = config.background.effective_mode().as_str(),
            background_image = background_path
                .as_deref()
                .map(|path| path.display().to_string()),
            "reloaded curtain config"
        );

        self.render_all_surfaces(queue_handle);
        self.maybe_start_background_render();
    }
}
