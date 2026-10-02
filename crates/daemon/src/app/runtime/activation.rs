mod attempt;
mod startup;

use super::state::{LockActivation, update_locked_hint};
use crate::{adapters::logind, domain::lock_state::LockState};
use anyhow::{Context, Result, anyhow};
use attempt::activate_lock_attempt;
use veila_common::{BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot, ipc::LatencyReportMode};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn activate_lock(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    initial_background_path: Option<&std::path::Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    force_emergency_ui: bool,
    latency_report: LatencyReportMode,
    acquire_timeout_seconds: u64,
    daemon_config_load_ms: u64,
    daemon_config_load_us: u64,
) -> Result<LockActivation> {
    let mut lock_was_confirmed = false;

    for emergency_retry in [false, true] {
        let result = activate_lock_attempt(
            trigger,
            session_proxy,
            state,
            if emergency_retry { None } else { config_path },
            if emergency_retry {
                None
            } else {
                initial_background_path
            },
            if emergency_retry {
                None
            } else {
                weather_snapshot
            },
            if emergency_retry {
                None
            } else {
                battery_snapshot
            },
            if emergency_retry {
                None
            } else {
                now_playing_snapshot
            },
            force_emergency_ui || emergency_retry,
            if emergency_retry {
                LatencyReportMode::Disabled
            } else {
                latency_report
            },
            acquire_timeout_seconds,
            daemon_config_load_ms,
            daemon_config_load_us,
        )
        .await;

        match result {
            Ok(activation) => return Ok(activation),
            Err(failure) => {
                lock_was_confirmed |= failure.session_locked;
                *state = if lock_was_confirmed {
                    LockState::Locked
                } else {
                    LockState::Unlocked
                };

                if !emergency_retry && failure.retry_safe {
                    tracing::error!(
                        session_locked = failure.session_locked,
                        "curtain failed before readiness: {:#}; retrying with emergency UI",
                        failure.error
                    );
                    continue;
                }

                update_locked_hint(session_proxy, lock_was_confirmed).await;
                return Err(failure.error).context(if emergency_retry {
                    "emergency curtain retry failed"
                } else {
                    "curtain activation failed and could not be retried safely"
                });
            }
        }
    }

    Err(anyhow!("curtain activation attempts exhausted"))
}
