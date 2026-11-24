use crate::{
    ipc::control::{ControlEvent, ControlSender},
    state::CurtainApp,
};
use anyhow::{Context, Result};
use std::{
    fs::File,
    io::BufReader,
    os::unix::net::UnixStream,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};
use veila_common::{
    WeatherSnapshot,
    session::{self, CurtainMessage, SupervisorMessage, frame},
};

pub(crate) struct Supervisor {
    pub config: veila_common::AppConfig,
    _lock: File,
    sender: SyncSender<CurtainMessage>,
    startup_weather: Receiver<WeatherSnapshot>,
    activity_at: std::cell::Cell<Option<std::time::Instant>>,
}
impl Supervisor {
    pub fn connect(
        channel: UnixStream,
        lock: File,
        control: ControlSender,
        config: veila_common::AppConfig,
    ) -> Result<Self> {
        let writer = channel.try_clone()?;
        let (sender, receiver) = mpsc::sync_channel(64);
        let (startup_weather_tx, startup_weather) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("veila-supervisor-read".into())
            .spawn(move || read_updates(channel, control, startup_weather_tx))?;
        thread::Builder::new()
            .name("veila-supervisor-write".into())
            .spawn(move || {
                let mut writer = writer;
                while let Ok(message) = receiver.recv() {
                    if frame::write(&mut writer, &message).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            config,
            activity_at: std::cell::Cell::new(None),
            _lock: lock,
            sender,
            startup_weather,
        })
    }
    pub fn send(&self, message: CurtainMessage) {
        if matches!(message, CurtainMessage::Activity) {
            if self
                .activity_at
                .get()
                .is_some_and(|last| last.elapsed() < std::time::Duration::from_millis(250))
            {
                return;
            }
            self.activity_at.set(Some(std::time::Instant::now()));
        }
        let _ = self.sender.try_send(message);
    }
}

fn read_updates(channel: UnixStream, control: ControlSender, startup: SyncSender<WeatherSnapshot>) {
    let mut reader = BufReader::new(channel);
    let mut startup = Some(startup);
    loop {
        let message = match frame::read::<SupervisorMessage>(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) => {
                tracing::warn!("supervisor disconnected; local PAM remains available");
                break;
            }
            Err(error) => {
                tracing::warn!(%error, "invalid supervisor channel; local PAM remains available");
                break;
            }
        };
        // only the first forecast belonging to the initial config can be sampled during rendering
        match &message {
            SupervisorMessage::ReloadConfig { .. } => startup = None,
            SupervisorMessage::UpdateWeather {
                snapshot: Some(snapshot),
            } => {
                if let Some(sender) = startup.take() {
                    let _ = sender.try_send(snapshot.clone());
                }
            }
            _ => {}
        }
        if control
            .send(ControlEvent::Supervisor(Box::new(message)))
            .is_err()
        {
            break;
        }
    }
    let _ = control.send(ControlEvent::UpdateFingerprintStatus { status: None });
}

impl CurtainApp {
    pub(crate) fn supervisor_event(
        &mut self,
        message: SupervisorMessage,
        queue: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match message {
            SupervisorMessage::ArmResumeInputGuard { request } => {
                // The one-way startup arm has no confirmed sleep transition.
                self.prepare_for_sleep(queue);
                self.notify_supervisor(CurtainMessage::ResumeGuardArmed { request });
            }
            SupervisorMessage::ReloadConfig { config } => self.apply_config(*config, queue),
            SupervisorMessage::Control(message) => {
                self.handle_control_event(crate::ipc::control::control_event(message), queue)
            }
            SupervisorMessage::UpdateWeather { snapshot } => {
                if let Some(supervisor) = &self.supervisor
                    && snapshot.is_some()
                {
                    let _ = supervisor.startup_weather.try_recv();
                }
                if self.update_weather_snapshot(snapshot) {
                    tracing::debug!(
                        startup_elapsed_us = crate::state::elapsed_us(self.startup_started_at),
                        "received weather widget change"
                    );
                    self.render_widget_surfaces(veila_ui::WidgetKind::Weather, queue);
                }
            }
            SupervisorMessage::UpdateBattery { snapshot } => {
                self.battery_snapshot = snapshot.clone();
                if let Some(supervisor) = &self.supervisor {
                    self.ui_shell
                        .set_battery_snapshot(crate::state::effective_battery_snapshot(
                            &supervisor.config,
                            snapshot,
                        ));
                }
                self.render_all_surfaces(queue);
            }
        }
    }

    pub(crate) fn apply_startup_weather(&mut self) {
        if !self.session_locked
            || self.ready_notified
            || self.ui_shell.emergency_active()
            || self
                .lock_surfaces
                .iter()
                .enumerate()
                .any(|(index, surface)| {
                    self.ui_visible_on_surface(index)
                        && surface
                            .shm_pool
                            .as_ref()
                            .is_some_and(|pool| pool.has_committed_frame())
                })
        {
            return;
        }

        if let Some(supervisor) = &self.supervisor
            && let Ok(snapshot) = supervisor.startup_weather.try_recv()
            && self.update_weather_snapshot(Some(snapshot))
        {
            tracing::debug!(
                startup_elapsed_us = crate::state::elapsed_us(self.startup_started_at),
                "included weather in the first UI frame"
            );
        }
    }

    fn update_weather_snapshot(&mut self, snapshot: Option<WeatherSnapshot>) -> bool {
        self.weather_snapshot = snapshot.clone();
        self.supervisor.as_ref().is_some_and(|supervisor| {
            self.ui_shell.set_weather_snapshot(
                crate::state::effective_weather_location(&supervisor.config),
                crate::state::effective_weather_snapshot(&supervisor.config, snapshot),
                supervisor.config.weather.unit,
            )
        })
    }

    pub(crate) fn notify_supervisor(&self, message: CurtainMessage) {
        if let Some(supervisor) = &self.supervisor {
            supervisor.send(message);
        }
    }
}

pub(crate) fn inherited(options: &mut crate::CurtainOptions) -> Result<Option<(UnixStream, File)>> {
    let Some(channel) = options.ipc_fd else {
        return Ok(None);
    };
    let stream = UnixStream::from(session::fd::take(channel)?);
    // reject inherited pipes and unrelated file types before any Wayland work
    stream
        .peer_addr()
        .context("supervisor descriptor is not a connected socket")?;
    let lock = File::from(session::fd::take(
        options
            .lock_fd
            .context("missing inherited lock descriptor")?,
    )?);
    anyhow::ensure!(
        lock.metadata()?.is_file(),
        "lock descriptor is not a regular file"
    );
    let mut reader = BufReader::new(std::io::stdin());
    options.resolved_config =
        Some(frame::read(&mut reader)?.context("missing resolved startup config")?);
    Ok(Some((stream, lock)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Shutdown;

    #[test]
    fn startup_weather_does_not_pass_a_config_reload() {
        let snapshot = WeatherSnapshot {
            temperature_celsius: 4,
            condition: veila_common::WeatherCondition::Overcast,
            fetched_at_unix: 1,
        };
        for reloaded in [false, true] {
            let (mut writer, reader) = UnixStream::pair().unwrap();
            let (sender, control) = mpsc::channel();
            let (wake, _source) = calloop::ping::make_ping().unwrap();
            let (initial_tx, initial_rx) = mpsc::sync_channel(1);
            if reloaded {
                frame::write(
                    &mut writer,
                    &SupervisorMessage::ReloadConfig {
                        config: Box::default(),
                    },
                )
                .unwrap();
            }
            let weather = SupervisorMessage::UpdateWeather {
                snapshot: Some(snapshot.clone()),
            };
            frame::write(&mut writer, &weather).unwrap();
            writer.shutdown(Shutdown::Write).unwrap();

            read_updates(reader, ControlSender::new(sender, wake), initial_tx);

            assert_eq!(
                initial_rx.try_recv().ok(),
                (!reloaded).then_some(snapshot.clone())
            );
            if reloaded {
                assert!(matches!(
                    control.try_recv(),
                    Ok(ControlEvent::Supervisor(message))
                        if matches!(*message, SupervisorMessage::ReloadConfig { .. })
                ));
            }
            assert_eq!(
                control.try_recv().unwrap(),
                ControlEvent::Supervisor(Box::new(weather))
            );
        }
    }
}
