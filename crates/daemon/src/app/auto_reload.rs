use std::path::Path;

use veila_common::AppConfig;

use super::{
    helpers::{apply_loaded_config, record_reload_result},
    runtime::control_socket_path,
    state::AppRuntime,
    watch::{AutoReloadTrigger, effective_auto_reload_debounce_ms},
};

impl AutoReloadTrigger {
    fn source(self) -> &'static str {
        match self {
            Self::Config => "config-change",
            Self::Include => "include-change",
            Self::Theme => "theme-change",
            Self::Wallpaper => "wallpaper-change",
        }
    }

    fn change_description(self) -> &'static str {
        match self {
            Self::Config => "config file change",
            Self::Include => "include file change",
            Self::Theme => "theme file change",
            Self::Wallpaper => "wallpaper change",
        }
    }
}

pub(super) async fn handle(
    trigger: AutoReloadTrigger,
    config_path: Option<&Path>,
    runtime: &mut AppRuntime,
) {
    let current_auto_reload = runtime.loaded_config.config.lock.auto_reload_config;
    let new_loaded_config = match AppConfig::load(config_path) {
        Ok(config) => config,
        Err(error) => {
            if trigger != AutoReloadTrigger::Config || current_auto_reload {
                record_failure(
                    runtime,
                    trigger,
                    &format!(
                        "failed to auto reload daemon config after {}: {error:#}",
                        trigger.change_description(),
                    ),
                );
            }
            return;
        }
    };
    // Config edits remain watched so they can re-enable automatic reload.
    if trigger == AutoReloadTrigger::Config
        && !current_auto_reload
        && !new_loaded_config.config.lock.auto_reload_config
    {
        tracing::debug!("ignoring config file change because auto_reload_config is disabled");
        return;
    }

    let debounce_ms = effective_auto_reload_debounce_ms(&new_loaded_config);
    let result = apply(runtime, new_loaded_config, trigger.source(), debounce_ms).await;
    if let Err(reason) = result {
        record_failure(runtime, trigger, &reason);
    }
}

async fn apply(
    runtime: &mut AppRuntime,
    new_loaded_config: veila_common::LoadedConfig,
    source: &str,
    debounce_ms: u64,
) -> Result<veila_common::ipc::DaemonReloadStatus, String> {
    apply_loaded_config(
        &runtime.state,
        control_socket_path(&runtime.active),
        &mut runtime.loaded_config,
        new_loaded_config,
        &mut runtime.last_reload_result,
        &mut runtime.last_reload_unix_ms,
        source,
        Some(debounce_ms),
        &mut runtime.auth_policy,
        &mut runtime.auth_state,
        &mut runtime.suspend_state,
        &runtime.weather,
        &runtime.battery,
        &runtime.now_playing,
    )
    .await
}

fn record_failure(runtime: &mut AppRuntime, trigger: AutoReloadTrigger, reason: &str) {
    record_reload_result(
        &mut runtime.last_reload_result,
        &mut runtime.last_reload_unix_ms,
        format!("error:{}:{reason}", trigger.source()),
    );
    tracing::warn!("{reason}");
}

#[cfg(test)]
mod tests;
