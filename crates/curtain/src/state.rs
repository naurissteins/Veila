use std::{
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, channel},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use kwylock_common::AppConfig;
use kwylock_renderer::{FrameSize, SoftwareBuffer, shm};
use kwylock_ui::{ShellAction, ShellKey, ShellState, ShellTheme};
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
    session_lock::{
        SessionLock, SessionLockState, SessionLockSurface, SessionLockSurfaceConfigure,
    },
    shm::Shm,
};

use crate::{
    CurtainOptions,
    auth::{AuthEvent, submit_password},
};

pub(crate) struct ManagedLockSurface {
    pub(crate) output: wl_output::WlOutput,
    pub(crate) surface: SessionLockSurface,
    pub(crate) size: Option<(u32, u32)>,
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
    notify_socket: Option<PathBuf>,
    daemon_socket: Option<PathBuf>,
    auth_events: Receiver<AuthEvent>,
    auth_sender: Sender<AuthEvent>,
    ui_shell: ShellState,
    lock_wait_timeout: Duration,
    lock_started_at: Instant,
    pub(crate) session_locked: bool,
    pub(crate) session_finished: bool,
    pub(crate) exit_requested: bool,
    ready_notified: bool,
    auth_in_flight: bool,
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
        let loaded_config = AppConfig::load(options.config_path.as_deref())
            .context("failed to load curtain config")?;
        let config = loaded_config.config;
        let ui_shell = ShellState::new(ShellTheme::from_config(&config));
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
            auth_events,
            auth_sender,
            ui_shell,
            lock_wait_timeout,
            lock_started_at: Instant::now(),
            session_locked: false,
            session_finished: false,
            exit_requested: false,
            ready_notified: false,
            auth_in_flight: false,
            has_keyboard_focus: false,
            failure_reason: None,
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
        if self.session_locked || self.session_finished {
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

    pub(crate) fn configure_surface(
        &mut self,
        queue_handle: &QueueHandle<Self>,
        surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
    ) {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface.wl_surface())
        else {
            tracing::warn!("configure received for unknown session-lock surface");
            return;
        };

        let size = self.resolve_surface_size(index, configure.new_size);
        self.lock_surfaces[index].size = Some(size);

        if let Err(error) = self.render_surface(&surface, size, queue_handle) {
            self.failure_reason = Some(format!("failed to render curtain surface: {error:#}"));
            self.exit_requested = true;
            return;
        }

        self.maybe_notify_ready();
    }

    pub(crate) fn render_all_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        let surfaces: Vec<_> = self
            .lock_surfaces
            .iter()
            .filter_map(|entry| entry.size.map(|size| (entry.surface.clone(), size)))
            .collect();

        for (surface, size) in surfaces {
            if let Err(error) = self.render_surface(&surface, size, queue_handle) {
                self.failure_reason = Some(format!("failed to rerender UI shell: {error:#}"));
                self.exit_requested = true;
                return;
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

            self.auth_in_flight = true;
            submit_password(socket_path, secret, self.auth_sender.clone());
        }
        self.render_all_surfaces(queue_handle);
    }

    pub(crate) fn drain_auth_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.auth_events.try_recv() {
            match event {
                AuthEvent::Rejected { retry_after_ms } => {
                    self.auth_in_flight = false;
                    self.ui_shell.authentication_rejected(retry_after_ms);
                    self.render_all_surfaces(queue_handle);
                }
                AuthEvent::Busy => {
                    self.auth_in_flight = false;
                    self.ui_shell.authentication_busy();
                    self.render_all_surfaces(queue_handle);
                }
            }
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

    fn render_surface(
        &mut self,
        surface: &SessionLockSurface,
        size: (u32, u32),
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(size.0, size.1))
            .map_err(|error| anyhow!("failed to allocate software buffer: {error}"))?;
        self.ui_shell.render(&mut buffer);
        shm::commit_buffer(&self.shm, queue_handle, surface.wl_surface(), &buffer)
            .map_err(|error| anyhow!("failed to commit software buffer: {error}"))
    }

    fn resolve_surface_size(&self, index: usize, requested: (u32, u32)) -> (u32, u32) {
        if requested.0 > 0 && requested.1 > 0 {
            return requested;
        }

        if let Some(info) = self.output_state.info(&self.lock_surfaces[index].output)
            && let Some((width, height)) = info.logical_size
            && width > 0
            && height > 0
        {
            tracing::warn!(
                output = info.name.as_deref().unwrap_or("unknown"),
                width,
                height,
                "lock surface configure had zero dimension; falling back to output logical size"
            );
            return (width as u32, height as u32);
        }

        tracing::warn!("lock surface configure had zero dimension; falling back to 1920x1080");
        (1920, 1080)
    }

    pub(crate) fn maybe_notify_ready(&mut self) {
        if self.ready_notified || !self.session_locked || self.lock_surfaces.is_empty() {
            return;
        }

        if self.lock_surfaces.iter().any(|entry| entry.size.is_none()) {
            return;
        }

        self.ready_notified = true;

        if let Some(path) = self.notify_socket.as_deref() {
            if let Err(error) = notify_ready(path) {
                tracing::warn!(?path, "failed to notify ready state: {error:#}");
            } else {
                tracing::info!(?path, "curtain reported readiness");
            }
        }
    }
}

fn notify_ready(path: &Path) -> Result<()> {
    use std::io::Write as _;
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(path)
        .with_context(|| format!("failed to connect to notify socket {}", path.display()))?;
    stream
        .write_all(&[1u8])
        .with_context(|| format!("failed to write readiness byte to {}", path.display()))?;

    Ok(())
}
