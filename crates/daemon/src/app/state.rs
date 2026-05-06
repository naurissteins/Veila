use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{
    net::UnixListener,
    process::Child,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use veila_common::LoadedConfig;
use veila_common::config::BackgroundSlideshowOrder;
use veila_common::ipc::LockPowerStatusSnapshot;

use crate::domain::{
    auth::{AuthPolicy, AuthState},
    lock_state::LockState,
};

use super::{
    battery::BatteryHandle,
    mpris::NowPlayingHandle,
    runtime::AuthResult,
    suspend::{LockedSuspendState, suspend_delay_seconds},
    weather::WeatherHandle,
};

pub(super) struct AppRuntime {
    pub(super) loaded_config: LoadedConfig,
    pub(super) last_reload_result: Option<String>,
    pub(super) last_reload_unix_ms: Option<u64>,
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
    pub(super) background_selection: Option<BackgroundSelectionState>,
    pub(super) suspend_state: LockedSuspendState,
    pub(super) last_power_status_snapshot: Option<LockPowerStatusSnapshot>,
    pub(super) power_status_sent: bool,
}

impl AppRuntime {
    pub(super) fn new(loaded_config: LoadedConfig) -> Self {
        let auth_policy = AuthPolicy::new(
            Duration::from_millis(loaded_config.config.lock.auth_backoff_base_ms),
            Duration::from_secs(loaded_config.config.lock.auth_backoff_max_seconds),
        );
        let suspend_delay = suspend_delay_seconds(&loaded_config.config).map(Duration::from_secs);
        let suspend_only_on_battery = loaded_config.config.lock.suspend_only_on_battery;
        let weather = WeatherHandle::spawn(&loaded_config.config.weather);
        let battery = BatteryHandle::spawn(&loaded_config.config.battery, suspend_only_on_battery);
        let now_playing = NowPlayingHandle::spawn(&loaded_config.config.now_playing);
        let skip_suspend_while_media_playing =
            loaded_config.config.lock.skip_suspend_while_media_playing;

        Self {
            loaded_config,
            last_reload_result: None,
            last_reload_unix_ms: None,
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
            background_selection: None,
            suspend_state: LockedSuspendState::new(
                suspend_delay,
                suspend_only_on_battery,
                skip_suspend_while_media_playing,
            ),
            last_power_status_snapshot: None,
            power_status_sent: false,
        }
    }

    pub(super) fn select_initial_background_path(&mut self) -> Option<PathBuf> {
        super::helpers::select_initial_background_path(
            &self.loaded_config.config,
            &mut self.background_selection,
        )
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

    pub(super) fn slots_with_policy_and_suspend(
        &mut self,
    ) -> (AuthPolicy, &mut LockedSuspendState, RuntimeSlots<'_>) {
        let Self {
            auth_policy,
            suspend_state,
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
            *auth_policy,
            suspend_state,
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

    pub(super) fn control_inputs(
        &mut self,
    ) -> (
        &mut LoadedConfig,
        &mut Option<String>,
        &mut Option<u64>,
        &mut AuthPolicy,
        &mut Option<BackgroundSelectionState>,
        &mut LockedSuspendState,
        RuntimeSlots<'_>,
    ) {
        let Self {
            loaded_config,
            last_reload_result,
            last_reload_unix_ms,
            auth_policy,
            background_selection,
            suspend_state,
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
            last_reload_unix_ms,
            auth_policy,
            background_selection,
            suspend_state,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct BackgroundSelectionState {
    paths: Vec<PathBuf>,
    remaining: Vec<PathBuf>,
    last: Option<PathBuf>,
    sequence_index: usize,
}

impl BackgroundSelectionState {
    pub(super) fn next_path(
        &mut self,
        paths: &[PathBuf],
        order: BackgroundSlideshowOrder,
    ) -> Option<PathBuf> {
        if paths.is_empty() {
            self.paths.clear();
            self.remaining.clear();
            self.last = None;
            self.sequence_index = 0;
            return None;
        }

        if self.paths != paths {
            self.paths = paths.to_vec();
            self.remaining.clear();
            self.last = self
                .last
                .take()
                .filter(|last| self.paths.iter().any(|path| path == last));
            self.sequence_index = 0;
        }

        match order {
            BackgroundSlideshowOrder::Sequence => {
                let next = self.paths[self.sequence_index % self.paths.len()].clone();
                self.sequence_index = (self.sequence_index + 1) % self.paths.len();
                self.last = Some(next.clone());
                Some(next)
            }
            BackgroundSlideshowOrder::Random => {
                if self.remaining.is_empty() {
                    self.remaining = self.paths.clone();
                    shuffle_paths(&mut self.remaining);
                    if self.remaining.len() > 1
                        && let Some(last) = self.last.as_ref()
                        && self.remaining.first() == Some(last)
                        && let Some(next_distinct_index) =
                            self.remaining.iter().position(|path| path != last)
                    {
                        self.remaining.swap(0, next_distinct_index);
                    }
                }

                let next = self.remaining.remove(0);
                self.last = Some(next.clone());
                Some(next)
            }
        }
    }
}

fn shuffle_paths(paths: &mut [PathBuf]) {
    let mut state = shuffle_seed();
    for index in (1..paths.len()).rev() {
        let candidate = next_u64(&mut state) as usize % (index + 1);
        paths.swap(index, candidate);
    }
}

fn shuffle_seed() -> u64 {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    seed ^ 0xA076_1D64_78BD_642F
}

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
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

#[cfg(test)]
mod tests {
    use veila_common::config::BackgroundSlideshowOrder;

    use super::BackgroundSelectionState;

    #[test]
    fn random_shuffle_cycle_avoids_immediate_repeats() {
        let paths = vec![
            "/tmp/one.jpg".into(),
            "/tmp/two.jpg".into(),
            "/tmp/three.jpg".into(),
        ];
        let mut selection = BackgroundSelectionState::default();
        let first = selection
            .next_path(&paths, BackgroundSlideshowOrder::Random)
            .expect("first path");
        let second = selection
            .next_path(&paths, BackgroundSlideshowOrder::Random)
            .expect("second path");
        let third = selection
            .next_path(&paths, BackgroundSlideshowOrder::Random)
            .expect("third path");
        let fourth = selection
            .next_path(&paths, BackgroundSlideshowOrder::Random)
            .expect("fourth path");

        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_ne!(third, fourth);
    }

    #[test]
    fn random_shuffle_resets_when_path_set_changes() {
        let first_paths = vec!["/tmp/one.jpg".into(), "/tmp/two.jpg".into()];
        let second_paths = vec!["/tmp/three.jpg".into(), "/tmp/four.jpg".into()];
        let mut selection = BackgroundSelectionState::default();
        let _ = selection
            .next_path(&first_paths, BackgroundSlideshowOrder::Random)
            .expect("first path");
        let next = selection
            .next_path(&second_paths, BackgroundSlideshowOrder::Random)
            .expect("changed path");

        assert!(second_paths.contains(&next));
    }

    #[test]
    fn sequence_selection_advances_per_lock() {
        let paths = vec![
            "/tmp/one.jpg".into(),
            "/tmp/two.jpg".into(),
            "/tmp/three.jpg".into(),
        ];
        let mut selection = BackgroundSelectionState::default();

        let first = selection
            .next_path(&paths, BackgroundSlideshowOrder::Sequence)
            .expect("first path");
        let second = selection
            .next_path(&paths, BackgroundSlideshowOrder::Sequence)
            .expect("second path");
        let third = selection
            .next_path(&paths, BackgroundSlideshowOrder::Sequence)
            .expect("third path");
        let fourth = selection
            .next_path(&paths, BackgroundSlideshowOrder::Sequence)
            .expect("fourth path");

        assert_eq!(first, paths[0]);
        assert_eq!(second, paths[1]);
        assert_eq!(third, paths[2]);
        assert_eq!(fourth, paths[0]);
    }
}
