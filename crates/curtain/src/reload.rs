use anyhow::Context;
use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_common::AppConfig;
use veila_renderer::background::BackgroundAsset;
use veila_ui::ShellTheme;
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1;

use crate::{
    background::BackgroundSlideshow,
    state::{
        CurtainApp, background_generated, background_treatment, effective_battery_snapshot,
        effective_weather_location, effective_weather_snapshot,
    },
};

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
            background_generated(&config.background),
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
        let background_generated = background_generated(&config.background);
        let slideshow = BackgroundSlideshow::load(&config.background, None);
        let background_path = slideshow
            .as_ref()
            .map(|slideshow| slideshow.current_path().to_path_buf())
            .or_else(|| config.background.resolved_path());

        self.background_color = theme.background;
        self.background_asset = background_asset;
        self.background_generated = background_generated;
        self.background_treatment = background_treatment(&config.background);
        self.background_path = background_path.clone();
        self.background_outputs = config.background.outputs.clone();
        self.slideshow = slideshow;
        self.ui_output_mode = config.visuals.output_ui_mode();
        self.ui_output_name = config.visuals.ui_output_name().map(str::to_owned);
        self.lock_wait_timeout =
            std::time::Duration::from_secs(config.lock.acquire_timeout_seconds.max(1));
        let screen_off_delay = config
            .lock
            .screen_off_seconds
            .filter(|seconds| *seconds > 0)
            .map(std::time::Duration::from_secs);
        let should_wake_outputs = self.outputs_powered_off() && screen_off_delay.is_none();
        self.screen_off
            .set_delay(screen_off_delay, std::time::Instant::now());
        if self.screen_off.enabled() && self.output_power_manager.get().is_err() {
            tracing::warn!(
                screen_off_seconds = config.lock.screen_off_seconds,
                "output power management is unavailable; locked screen-off timer is disabled"
            );
        }
        for index in 0..self.lock_surfaces.len() {
            if self.screen_off.enabled() {
                if self.lock_surfaces[index].output_power.is_none() {
                    let output = self.lock_surfaces[index].output.clone();
                    self.lock_surfaces[index].output_power =
                        self.bind_output_power_for_surface(&output, queue_handle);
                }
                continue;
            }

            if let Some(output_power) = self.lock_surfaces[index].output_power.take() {
                output_power.destroy();
            }
        }
        if self.outputs_powered_off() && self.screen_off.enabled() {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::Off);
        }
        let avatar_path = config.avatar_image_path().map(std::path::Path::to_path_buf);
        self.avatar_path = avatar_path.clone();
        self.avatar_load_started = false;
        self.ui_shell.apply_theme_with_username_and_weather(
            theme,
            Some(config.visuals.input_placeholder()),
            config.visuals.username_text().map(str::to_owned),
            avatar_path,
            config.visuals.username_enabled(),
            effective_weather_location(&config),
            effective_weather_snapshot(&config, self.weather_snapshot.clone()),
            config.weather.unit,
            effective_battery_snapshot(&config, self.battery_snapshot.clone()),
            self.now_playing_snapshot.clone(),
        );
        self.reset_background_source_state();
        if should_wake_outputs {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
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
            background_output_overrides = config.background.outputs.len(),
            background_slideshow_images = self.slideshow.as_ref().map(BackgroundSlideshow::len),
            "reloaded curtain config"
        );

        self.render_all_surfaces(queue_handle);
        self.maybe_start_background_render();
    }
}
