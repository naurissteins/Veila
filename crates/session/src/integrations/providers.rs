use super::{Output, State};
use crate::services::{
    battery::BatteryHandle,
    mpris::NowPlayingHandle,
    watch::{AutoReloadTrigger, AutoReloadWatcher},
    weather::WeatherHandle,
};
use std::path::Path;
use veila_common::{LoadedConfig, session::SupervisorMessage};

pub(super) struct Providers {
    pub battery: BatteryHandle,
    pub media: NowPlayingHandle,
    weather: WeatherHandle,
    weather_config: veila_common::WeatherConfig,
    watcher: AutoReloadWatcher,
    last: Vec<SupervisorMessage>,
    generation: u64,
}
impl Providers {
    pub fn new(state: &State, path: Option<&Path>) -> Self {
        let config = &state.config;
        Self {
            battery: BatteryHandle::spawn(&config.battery, config.lock.suspend_only_on_battery),
            media: NowPlayingHandle::spawn(&config.now_playing),
            weather: WeatherHandle::spawn(&config.weather),
            weather_config: config.weather.clone(),
            watcher: AutoReloadWatcher::new(
                path,
                &LoadedConfig {
                    path: path.map(Path::to_path_buf),
                    config: config.clone(),
                },
            ),
            last: Vec::new(),
            generation: state.generation,
        }
    }
    pub fn update(&mut self, state: &State) {
        self.last.clear();
        self.battery.update_config(
            &state.config.battery,
            state.config.lock.suspend_only_on_battery,
        );
        self.media.update_config(&state.config.now_playing);
        if self.weather_config != state.config.weather {
            // Replacing the reader prevents an old-location fetch from reaching the new config.
            self.weather = WeatherHandle::spawn(&state.config.weather);
            self.weather_config = state.config.weather.clone();
        }
    }
    pub async fn refresh(&mut self, state: &State, path: Option<&Path>, output: &mut Output) {
        if !state.confirmed {
            return;
        }
        let messages = vec![
            SupervisorMessage::UpdateBattery {
                snapshot: self.battery.current_snapshot(),
            },
            SupervisorMessage::UpdateWeather {
                snapshot: self.weather.current_snapshot(),
            },
            SupervisorMessage::Control(
                veila_common::ipc::CurtainControlMessage::UpdateNowPlaying {
                    snapshot: self.media.current_snapshot(),
                },
            ),
        ];
        if self.generation != state.generation {
            self.last.clear();
            self.generation = state.generation;
        }
        for (index, message) in messages.iter().enumerate() {
            if self.last.get(index) != Some(message) {
                output
                    .send(super::Event::Message {
                        generation: state.generation,
                        revision: Some(state.revision),
                        message: message.clone(),
                    })
                    .await;
            }
        }
        self.last = messages;
        let loaded = LoadedConfig {
            path: path.map(Path::to_path_buf),
            config: state.config.clone(),
        };
        if let Some(trigger) = self.watcher.poll(path, &loaded) {
            match veila_common::AppConfig::load(path) {
                Ok(loaded)
                    if state.config.lock.auto_reload_config
                        || (trigger == AutoReloadTrigger::Config
                            && loaded.config.lock.auto_reload_config) =>
                {
                    if veila_common::session::frame::encode(&loaded.config).is_ok() {
                        output
                            .send(super::Event::Reload {
                                generation: state.generation,
                                revision: state.revision,
                                config: Box::new(loaded.config),
                            })
                            .await;
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%error, "automatic reload rejected; retaining current config")
                }
            }
        }
    }
}
