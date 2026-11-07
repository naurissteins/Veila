use crate::{
    ipc::control::{ControlEvent, ControlSender},
    state::CurtainApp,
};
use anyhow::{Context, Result};
use std::{
    fs::File,
    io::BufReader,
    os::unix::net::UnixStream,
    sync::mpsc::{self, SyncSender},
    thread,
};
use veila_common::{
    ipc::CurtainControlMessage,
    session::{self, CurtainMessage, SupervisorMessage, frame},
};

pub(crate) struct Supervisor {
    pub config: veila_common::AppConfig,
    _lock: File,
    sender: SyncSender<CurtainMessage>,
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
        thread::Builder::new().name("veila-supervisor-read".into()).spawn(move||{
            let mut reader=BufReader::new(channel);
            loop {
                match frame::read::<SupervisorMessage>(&mut reader) {
                    Ok(Some(message))=>{if control.send(ControlEvent::Supervisor(Box::new(message))).is_err(){break;}},
                    Ok(None)=>{
                        tracing::warn!("supervisor disconnected; local PAM remains available");break;
                    },
                    Err(error)=>{tracing::warn!(%error,"invalid supervisor channel; local PAM remains available");break;},
                }
            }
            let _ = control.send(ControlEvent::UpdateFingerprintStatus { status: None });
        })?;
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

impl CurtainApp {
    pub(crate) fn supervisor_event(
        &mut self,
        message: SupervisorMessage,
        queue: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match message {
            SupervisorMessage::ArmResumeInputGuard { request } => {
                self.handle_control_event(ControlEvent::ArmResumeInputGuard, queue);
                self.notify_supervisor(CurtainMessage::ResumeGuardArmed { request });
            }
            SupervisorMessage::ReloadConfig { config } => self.apply_config(*config, queue),
            SupervisorMessage::Control(CurtainControlMessage::ReloadConfig) => {
                tracing::warn!("ignoring disk reload on resolved supervisor channel");
            }
            SupervisorMessage::Control(message) => {
                self.handle_control_event(crate::ipc::control::control_event(message), queue)
            }
            SupervisorMessage::UpdateWeather { snapshot } => {
                self.weather_snapshot = snapshot.clone();
                if let Some(supervisor) = &self.supervisor {
                    self.ui_shell.set_weather_snapshot(
                        crate::state::effective_weather_location(&supervisor.config),
                        crate::state::effective_weather_snapshot(&supervisor.config, snapshot),
                        supervisor.config.weather.unit,
                    );
                }
                self.render_all_surfaces(queue);
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
    let snapshots: veila_common::ipc::CurtainInitialSnapshots =
        frame::read(&mut reader)?.context("missing startup snapshots")?;
    options.weather_snapshot = snapshots.weather;
    options.battery_snapshot = snapshots.battery;
    options.now_playing_snapshot = snapshots.now_playing;
    Ok(Some((stream, lock)))
}
