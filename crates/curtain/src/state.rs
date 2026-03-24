use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{
        Connection, QueueHandle,
        globals::GlobalList,
        protocol::{wl_keyboard, wl_output},
    },
    registry::RegistryState,
    seat::SeatState,
    session_lock::{SessionLock, SessionLockState, SessionLockSurface},
    shm::Shm,
};
use veila_common::AppConfig;
use veila_renderer::{
    ClearColor,
    background::{BackgroundAsset, BackgroundTreatment},
};
use veila_ui::{ShellAction, ShellKey, ShellState, ShellTheme};

use crate::{
    CurtainOptions,
    background::BackgroundEvent,
    ipc::auth::{AuthEvent, submit_password},
    ipc::control::{ControlEvent, spawn_listener},
};

pub(crate) struct ManagedLockSurface {
    pub(crate) output: wl_output::WlOutput,
    pub(crate) surface: SessionLockSurface,
    pub(crate) size: Option<(u32, u32)>,
    pub(crate) background: Option<veila_renderer::SoftwareBuffer>,
}

pub(crate) struct CurtainApp {
    pub(crate) connection: Connection,
    pub(crate) compositor_state: CompositorState,
    pub(crate) output_state: OutputState,
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) session_lock_state: SessionLockState,
    pub(crate) session_lock: Option<SessionLock>,
    pub(crate) shm: Shm,
    pub(crate) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(crate) lock_surfaces: Vec<ManagedLockSurface>,
    pub(crate) notify_socket: Option<PathBuf>,
    daemon_socket: Option<PathBuf>,
    control_socket: Option<PathBuf>,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) background_path: Option<PathBuf>,
    auth_events: Receiver<AuthEvent>,
    auth_sender: Sender<AuthEvent>,
    pub(crate) background_sender: Sender<BackgroundEvent>,
    pub(crate) background_events: Receiver<BackgroundEvent>,
    control_events: Receiver<ControlEvent>,
    pub(crate) background_asset: BackgroundAsset,
    pub(crate) background_treatment: BackgroundTreatment,
    pub(crate) background_color: ClearColor,
    pub(crate) ui_shell: ShellState,
    pub(crate) lock_wait_timeout: Duration,
    lock_started_at: Instant,
    lock_acquisition_started: bool,
    pub(crate) session_locked: bool,
    pub(crate) session_finished: bool,
    pub(crate) exit_requested: bool,
    pub(crate) ready_notified: bool,
    pub(crate) background_render_started: bool,
    auth_in_flight: bool,
    next_auth_attempt_id: u64,
    pub(crate) has_keyboard_focus: bool,
    pub(crate) failure_reason: Option<String>,
}

impl CurtainApp {
    pub(crate) fn new(
        connection: Connection,
        globals: &GlobalList,
        queue_handle: &QueueHandle<Self>,
        options: CurtainOptions,
    ) -> Result<Self> {
        let (auth_sender, auth_events) = channel();
        let (background_sender, background_events) = channel();
        let (control_sender, control_events) = channel();
        let loaded_config = AppConfig::load(options.config_path.as_deref())
            .context("failed to load curtain config")?;
        let config = loaded_config.config;
        let theme = ShellTheme::from_config(&config);
        let background_color = theme.background;
        let background_asset = BackgroundAsset::load(
            None,
            background_color,
            background_treatment(&config.background),
        )
        .context("failed to prepare fallback background")?;
        let background_treatment = background_treatment(&config.background);
        let ui_shell = ShellState::new(
            theme,
            config.lock.user_hint.clone(),
            config.lock.avatar_path.clone(),
        );
        let lock_wait_timeout = Duration::from_secs(config.lock.acquire_timeout_seconds.max(1));

        tracing::info!(
            config = loaded_config
                .path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "defaults".to_string()),
            background_image = config
                .background
                .path
                .as_deref()
                .map(|path| path.display().to_string()),
            "loaded curtain config"
        );

        if let Some(control_socket) = options.control_socket.clone() {
            spawn_listener(control_socket, control_sender)
                .context("failed to start curtain control listener")?;
        }

        Ok(Self {
            connection,
            compositor_state: CompositorState::bind(globals, queue_handle)
                .context("compositor does not advertise wl_compositor")?,
            output_state: OutputState::new(globals, queue_handle),
            registry_state: RegistryState::new(globals),
            seat_state: SeatState::new(globals, queue_handle),
            session_lock_state: SessionLockState::new(globals, queue_handle),
            session_lock: None,
            shm: Shm::bind(globals, queue_handle)
                .context("compositor does not advertise wl_shm")?,
            keyboard: None,
            lock_surfaces: Vec::new(),
            notify_socket: options.notify_socket,
            daemon_socket: options.daemon_socket,
            control_socket: options.control_socket,
            config_path: options.config_path,
            background_path: config.background.path,
            auth_events,
            auth_sender,
            background_sender,
            background_events,
            control_events,
            background_asset,
            background_treatment,
            background_color,
            ui_shell,
            lock_wait_timeout,
            lock_started_at: Instant::now(),
            session_locked: false,
            session_finished: false,
            exit_requested: false,
            ready_notified: false,
            background_render_started: false,
            auth_in_flight: false,
            next_auth_attempt_id: 1,
            has_keyboard_focus: false,
            failure_reason: None,
            lock_acquisition_started: false,
        })
    }

    pub(crate) fn acquire_lock(&mut self, queue_handle: &QueueHandle<Self>) -> Result<()> {
        let outputs: Vec<_> = self.output_state.outputs().collect();
        if outputs.is_empty() {
            bail!("no Wayland outputs found");
        }

        let session_lock = self
            .session_lock_state
            .lock(queue_handle)
            .context("compositor does not support ext-session-lock-v1")?;
        self.session_lock = Some(session_lock);
        self.lock_started_at = Instant::now();
        self.lock_acquisition_started = true;

        for output in outputs {
            self.create_surface_for_output(output, queue_handle)?;
        }

        tracing::info!(surfaces = self.lock_surfaces.len(), "created lock surfaces");
        Ok(())
    }

    pub(crate) fn create_surface_for_output(
        &mut self,
        output: wl_output::WlOutput,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        if self
            .lock_surfaces
            .iter()
            .any(|entry| entry.output == output)
        {
            return Ok(());
        }

        let Some(session_lock) = self.session_lock.as_ref() else {
            return Ok(());
        };

        let wl_surface = self.compositor_state.create_surface(queue_handle);
        let surface = session_lock.create_lock_surface(wl_surface, &output, queue_handle);
        self.lock_surfaces.push(ManagedLockSurface {
            output,
            surface,
            size: None,
            background: None,
        });

        Ok(())
    }

    pub(crate) fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    pub(crate) fn can_stop(&self) -> bool {
        self.failure_reason.is_some()
            || (self.exit_requested && (self.session_locked || self.session_finished))
    }

    pub(crate) fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    pub(crate) fn check_lock_deadline(&mut self) -> Result<()> {
        if !self.lock_acquisition_started || self.session_locked || self.session_finished {
            return Ok(());
        }

        if self.lock_started_at.elapsed() <= self.lock_wait_timeout {
            return Ok(());
        }

        self.failure_reason =
            Some("timed out waiting for compositor to confirm the session lock".to_string());
        Err(anyhow!(
            "timed out waiting for compositor to confirm the session lock"
        ))
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        if let Some(path) = self.control_socket.take() {
            let _ = std::fs::remove_file(path);
        }

        if self.session_finished {
            self.session_lock.take();
            return Ok(());
        }

        if let Some(session_lock) = self.session_lock.take()
            && session_lock.is_locked()
        {
            tracing::info!("releasing session lock");
            session_lock.unlock();
            self.connection
                .roundtrip()
                .context("failed to roundtrip after unlocking session")?;
        }

        Ok(())
    }

    pub(crate) fn drain_control_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.control_events.try_recv() {
            match event {
                ControlEvent::Lock {
                    notify_socket,
                    daemon_socket,
                } => {
                    tracing::info!("received lock-now from daemon; acquiring session lock");
                    self.notify_socket = Some(notify_socket);
                    self.daemon_socket = Some(daemon_socket);
                    if let Err(error) = self.acquire_lock(queue_handle) {
                        self.failure_reason =
                            Some(format!("failed to acquire lock after standby: {error:#}"));
                        self.exit_requested = true;
                    }
                }
                ControlEvent::Unlock { attempt_id } => {
                    if let Some(attempt_id) = attempt_id {
                        tracing::info!(attempt_id, "received curtain unlock request from daemon");
                    } else {
                        tracing::info!("received curtain unlock request from daemon");
                    }
                    self.request_exit();
                }
                ControlEvent::Reload => {
                    tracing::info!("received curtain reload request from daemon");
                    self.reload_config(queue_handle);
                }
            }
        }
    }

    pub(crate) fn set_keyboard_focus(&mut self, focused: bool, queue_handle: &QueueHandle<Self>) {
        if self.has_keyboard_focus == focused {
            return;
        }

        self.has_keyboard_focus = focused;
        self.ui_shell.set_focus(focused);
        self.render_all_surfaces(queue_handle);
    }

    pub(crate) fn handle_shell_key(&mut self, key: ShellKey, queue_handle: &QueueHandle<Self>) {
        if self.auth_in_flight {
            return;
        }

        let action = self.ui_shell.handle_key(key);
        if let ShellAction::Submit(secret) = action {
            let Some(socket_path) = self.daemon_socket.clone() else {
                tracing::warn!("password submitted without a daemon auth socket");
                self.ui_shell.authentication_rejected(None);
                self.render_all_surfaces(queue_handle);
                return;
            };

            let attempt_id = self.next_auth_attempt_id;
            self.next_auth_attempt_id = self.next_auth_attempt_id.saturating_add(1);
            tracing::info!(
                attempt_id,
                secret_len = secret.chars().count(),
                "submitting password attempt"
            );
            self.auth_in_flight = true;
            submit_password(socket_path, attempt_id, secret, self.auth_sender.clone());
        }
        self.render_all_surfaces(queue_handle);
    }

    pub(crate) fn drain_auth_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.auth_events.try_recv() {
            match event {
                AuthEvent::Accepted { attempt_id } => {
                    tracing::info!(
                        attempt_id,
                        "waiting for daemon-driven unlock after auth success"
                    );
                }
                AuthEvent::Rejected {
                    attempt_id,
                    retry_after_ms,
                } => {
                    self.auth_in_flight = false;
                    tracing::info!(attempt_id, "updating UI after authentication rejection");
                    self.ui_shell.authentication_rejected(retry_after_ms);
                    self.render_all_surfaces(queue_handle);
                }
                AuthEvent::Busy { attempt_id } => {
                    self.auth_in_flight = false;
                    tracing::debug!(attempt_id, "updating UI after authentication busy response");
                    self.ui_shell.authentication_busy();
                    self.render_all_surfaces(queue_handle);
                }
            }
        }
    }

    pub(crate) fn advance_animated_scene(&mut self, queue_handle: &QueueHandle<Self>) {
        if self.ui_shell.advance_animated_state() {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn surface_has_focus_target(
        &self,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
    ) -> bool {
        self.lock_surfaces
            .iter()
            .any(|entry| entry.surface.wl_surface() == surface)
    }
}

pub(crate) fn background_treatment(
    config: &veila_common::config::BackgroundConfig,
) -> BackgroundTreatment {
    BackgroundTreatment {
        blur_radius: config.blur_radius,
        dim_strength: config.dim_strength,
        tint: config
            .tint
            .map(|color| ClearColor::opaque(color.0, color.1, color.2)),
        tint_opacity: config.tint_opacity,
    }
}
