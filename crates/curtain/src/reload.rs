use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_common::AppConfig;
use veila_ui::ShellTheme;
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1;

use crate::state::{
    CurtainApp, effective_battery_snapshot, effective_weather_location, effective_weather_snapshot,
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

        // Wallpaper is fixed for this lock session; only theme/widgets refresh here.
        self.background_color = theme.background;
        self.ui_output_mode = config.visuals.output_ui_mode();
        self.ui_output_name = config.visuals.ui_output_name().map(str::to_owned);
        self.hide_cursor = config.lock.hide_cursor;
        self.set_configured_pointer_cursor(&self.connection);
        self.allow_empty_password = config.lock.allow_empty_password;
        self.power_off_secondary_outputs = config.lock.power_off_secondary_outputs;
        self.lock_wait_timeout =
            std::time::Duration::from_secs(config.lock.acquire_timeout_seconds.max(1));
        let screen_off_delay = config
            .lock
            .screen_off_seconds
            .filter(|seconds| *seconds > 0)
            .map(std::time::Duration::from_secs);
        let should_wake_outputs = self.outputs_powered_off() && screen_off_delay.is_none();
        let should_wake_secondary_outputs = self.secondary_outputs_powered_off;
        self.screen_off
            .set_delay(screen_off_delay, std::time::Instant::now());
        if self.output_power_control_enabled() && self.output_power_manager.get().is_err() {
            tracing::warn!(
                screen_off_seconds = config.lock.screen_off_seconds,
                power_off_secondary_outputs = self.power_off_secondary_outputs,
                "output power management is unavailable; locked output power features are disabled"
            );
        }
        if (should_wake_outputs || should_wake_secondary_outputs)
            && !self.output_power_control_enabled()
        {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
        }
        for index in 0..self.lock_surfaces.len() {
            if self.output_power_control_enabled() {
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
        if should_wake_secondary_outputs {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
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
        if should_wake_outputs {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
        }

        tracing::info!(
            config = loaded_config
                .path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "defaults".to_string()),
            "reloaded curtain config (background preserved)"
        );

        self.render_all_surfaces(queue_handle);
        self.maybe_power_off_secondary_outputs();
    }
}
