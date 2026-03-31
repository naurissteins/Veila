use std::path::PathBuf;
use std::time::Duration;

use tokio::{
    net::UnixListener,
    process::Child,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use veila_common::LoadedConfig;

use crate::domain::{
    auth::{AuthPolicy, AuthState},
    lock_state::LockState,
};

use super::{
    battery::BatteryHandle, mpris::NowPlayingHandle, runtime::AuthResult, weather::WeatherHandle,
};

pub(super) struct AppRuntime {
    pub(super) loaded_config: LoadedConfig,
    pub(super) last_reload_result: Option<String>,
    pub(super) auth_policy: AuthPolicy,
    pub(super) weather: WeatherHandle,
    pub(super) battery: BatteryHandle,
    pub(super) now_playing: NowPlayingHandle,
    pub(super) state: LockState,
    pub(super) curtain: Option<Child>,
    pub(super) auth_listener: Option<UnixListener>,
    pub(super) auth_socket_path: Option<PathBuf>,
    pub(super) control_socket_path: Option<PathBuf>,
    pub(super) auth_results: Option<UnboundedReceiver<AuthResult>>,
    pub(super) auth_sender: Option<UnboundedSender<AuthResult>>,
    pub(super) auth_state: AuthState,
}

impl AppRuntime {
    pub(super) fn new(loaded_config: LoadedConfig) -> Self {
        let auth_policy = AuthPolicy::new(
            Duration::from_millis(loaded_config.config.lock.auth_backoff_base_ms),
            Duration::from_secs(loaded_config.config.lock.auth_backoff_max_seconds),
        );
        let weather = WeatherHandle::spawn(&loaded_config.config.weather);
        let battery = BatteryHandle::spawn(&loaded_config.config.battery);
        let now_playing = NowPlayingHandle::spawn();

        Self {
            loaded_config,
            last_reload_result: None,
            auth_policy,
            weather,
            battery,
            now_playing,
            state: LockState::Unlocked,
            curtain: None,
            auth_listener: None,
            auth_socket_path: None,
            control_socket_path: None,
            auth_results: None,
            auth_sender: None,
            auth_state: AuthState::new(auth_policy),
        }
    }

    pub(super) fn slots(&mut self) -> RuntimeSlots<'_> {
        RuntimeSlots {
            state: &mut self.state,
            curtain: &mut self.curtain,
            auth_listener: &mut self.auth_listener,
            auth_socket_path: &mut self.auth_socket_path,
            control_socket_path: &mut self.control_socket_path,
            auth_results: &mut self.auth_results,
            auth_sender: &mut self.auth_sender,
            auth_state: &mut self.auth_state,
        }
    }

    pub(super) fn slots_with_policy(&mut self) -> (AuthPolicy, RuntimeSlots<'_>) {
        (self.auth_policy, self.slots())
    }

    pub(super) fn control_inputs(
        &mut self,
    ) -> (
        &mut LoadedConfig,
        &mut Option<String>,
        &mut AuthPolicy,
        RuntimeSlots<'_>,
    ) {
        let Self {
            loaded_config,
            last_reload_result,
            auth_policy,
            state,
            curtain,
            auth_listener,
            auth_socket_path,
            control_socket_path,
            auth_results,
            auth_sender,
            auth_state,
            ..
        } = self;

        (
            loaded_config,
            last_reload_result,
            auth_policy,
            RuntimeSlots {
                state,
                curtain,
                auth_listener,
                auth_socket_path,
                control_socket_path,
                auth_results,
                auth_sender,
                auth_state,
            },
        )
    }
}

pub(super) struct RuntimeSlots<'a> {
    pub(super) state: &'a mut LockState,
    pub(super) curtain: &'a mut Option<Child>,
    pub(super) auth_listener: &'a mut Option<UnixListener>,
    pub(super) auth_socket_path: &'a mut Option<PathBuf>,
    pub(super) control_socket_path: &'a mut Option<PathBuf>,
    pub(super) auth_results: &'a mut Option<UnboundedReceiver<AuthResult>>,
    pub(super) auth_sender: &'a mut Option<UnboundedSender<AuthResult>>,
    pub(super) auth_state: &'a mut AuthState,
}

impl<'a>
    From<(
        &'a mut LockState,
        &'a mut Option<Child>,
        &'a mut Option<UnixListener>,
        &'a mut Option<PathBuf>,
        &'a mut Option<PathBuf>,
        &'a mut Option<UnboundedReceiver<AuthResult>>,
        &'a mut Option<UnboundedSender<AuthResult>>,
        &'a mut AuthState,
    )> for RuntimeSlots<'a>
{
    fn from(
        (
            state,
            curtain,
            auth_listener,
            auth_socket_path,
            control_socket_path,
            auth_results,
            auth_sender,
            auth_state,
        ): (
            &'a mut LockState,
            &'a mut Option<Child>,
            &'a mut Option<UnixListener>,
            &'a mut Option<PathBuf>,
            &'a mut Option<PathBuf>,
            &'a mut Option<UnboundedReceiver<AuthResult>>,
            &'a mut Option<UnboundedSender<AuthResult>>,
            &'a mut AuthState,
        ),
    ) -> Self {
        Self {
            state,
            curtain,
            auth_listener,
            auth_socket_path,
            control_socket_path,
            auth_results,
            auth_sender,
            auth_state,
        }
    }
}
