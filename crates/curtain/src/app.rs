use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use calloop::signals::{Signal, Signals};
use kwylock_renderer::{ClearColor, FrameSize, SoftwareBuffer, shm};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, Proxy, QueueHandle,
        globals::{GlobalList, registry_queue_init},
        protocol::{wl_buffer, wl_output, wl_surface},
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shm::{Shm, ShmHandler},
};

use crate::CurtainOptions;

const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

pub fn run(options: CurtainOptions) -> Result<()> {
    let connection =
        Connection::connect_to_env().context("failed to connect to Wayland display")?;
    let (globals, event_queue) =
        registry_queue_init(&connection).context("failed to enumerate Wayland globals")?;
    let queue_handle = event_queue.handle();

    let mut event_loop = smithay_client_toolkit::reexports::calloop::EventLoop::try_new()
        .context("failed to create curtain event loop")?;
    let loop_handle = event_loop.handle();
    let mut app = CurtainApp::new(connection.clone(), &globals, &queue_handle, options)?;
    app.acquire_lock(&queue_handle)?;

    let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM])
        .context("failed to register signal source")?;
    loop_handle
        .insert_source(signals, |event, _, app: &mut CurtainApp| {
            tracing::info!(?event, "termination requested");
            app.request_exit();
        })
        .context("failed to insert signal source into event loop")?;

    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(
        connection,
        event_queue,
    )
    .insert(loop_handle)
    .context("failed to insert Wayland source into event loop")?;

    while !app.can_stop() {
        event_loop
            .dispatch(Duration::from_millis(250), &mut app)
            .context("curtain event loop failed")?;
        app.check_lock_deadline()?;
    }

    app.shutdown()?;

    if let Some(reason) = app.failure_reason() {
        bail!(reason.to_string());
    }

    Ok(())
}

struct ManagedLockSurface {
    output: wl_output::WlOutput,
    surface: SessionLockSurface,
    configured: bool,
}

struct CurtainApp {
    connection: Connection,
    compositor_state: CompositorState,
    output_state: OutputState,
    registry_state: RegistryState,
    session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    shm: Shm,
    lock_surfaces: Vec<ManagedLockSurface>,
    notify_socket: Option<PathBuf>,
    lock_started_at: Instant,
    session_locked: bool,
    session_finished: bool,
    exit_requested: bool,
    ready_notified: bool,
    failure_reason: Option<String>,
}

impl CurtainApp {
    fn new(
        connection: Connection,
        globals: &GlobalList,
        queue_handle: &QueueHandle<Self>,
        options: CurtainOptions,
    ) -> Result<Self> {
        Ok(Self {
            connection,
            compositor_state: CompositorState::bind(globals, queue_handle)
                .context("compositor does not advertise wl_compositor")?,
            output_state: OutputState::new(globals, queue_handle),
            registry_state: RegistryState::new(globals),
            session_lock_state: SessionLockState::new(globals, queue_handle),
            session_lock: None,
            shm: Shm::bind(globals, queue_handle)
                .context("compositor does not advertise wl_shm")?,
            lock_surfaces: Vec::new(),
            notify_socket: options.notify_socket,
            lock_started_at: Instant::now(),
            session_locked: false,
            session_finished: false,
            exit_requested: false,
            ready_notified: false,
            failure_reason: None,
        })
    }

    fn acquire_lock(&mut self, queue_handle: &QueueHandle<Self>) -> Result<()> {
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

    fn create_surface_for_output(
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
            configured: false,
        });

        Ok(())
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    fn can_stop(&self) -> bool {
        self.failure_reason.is_some()
            || (self.exit_requested && (self.session_locked || self.session_finished))
    }

    fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    fn check_lock_deadline(&mut self) -> Result<()> {
        if self.session_locked || self.session_finished {
            return Ok(());
        }

        if self.lock_started_at.elapsed() <= LOCK_WAIT_TIMEOUT {
            return Ok(());
        }

        self.failure_reason =
            Some("timed out waiting for compositor to confirm the session lock".to_string());
        Err(anyhow!(
            "timed out waiting for compositor to confirm the session lock"
        ))
    }

    fn shutdown(&mut self) -> Result<()> {
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

    fn configure_surface(
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
        let buffer = match SoftwareBuffer::solid(
            FrameSize::new(size.0, size.1),
            ClearColor::opaque(0, 0, 0),
        ) {
            Ok(buffer) => buffer,
            Err(error) => {
                self.failure_reason = Some(format!(
                    "failed to build curtain software buffer: {error:#}"
                ));
                self.exit_requested = true;
                return;
            }
        };

        if let Err(error) =
            shm::commit_buffer(&self.shm, queue_handle, surface.wl_surface(), &buffer)
        {
            self.failure_reason = Some(format!("failed to render curtain surface: {error:#}"));
            self.exit_requested = true;
            return;
        }

        self.lock_surfaces[index].configured = true;
        self.maybe_notify_ready();
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

    fn maybe_notify_ready(&mut self) {
        if self.ready_notified || !self.session_locked || self.lock_surfaces.is_empty() {
            return;
        }

        if self.lock_surfaces.iter().any(|entry| !entry.configured) {
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

fn notify_ready(path: &std::path::Path) -> Result<()> {
    use std::io::Write as _;
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(path)
        .with_context(|| format!("failed to connect to notify socket {}", path.display()))?;
    stream
        .write_all(&[1u8])
        .with_context(|| format!("failed to write readiness byte to {}", path.display()))?;

    Ok(())
}

impl SessionLockHandler for CurtainApp {
    fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        tracing::info!("session lock confirmed by compositor");
        self.session_locked = true;
        self.maybe_notify_ready();
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        tracing::warn!("compositor denied or revoked the session lock");
        self.session_finished = true;
        self.failure_reason = Some("compositor denied or revoked the session lock".to_string());
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        self.configure_surface(queue_handle, surface, configure);
    }
}

impl OutputHandler for CurtainApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Err(error) = self.create_surface_for_output(output.clone(), queue_handle) {
            self.failure_reason = Some(format!(
                "failed to create session-lock surface for new output: {error:#}"
            ));
            self.exit_requested = true;
            return;
        }

        tracing::info!(
            id = output.id().protocol_id(),
            "registered new output while locked"
        );
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _queue_handle: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _queue_handle: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.lock_surfaces.retain(|entry| entry.output != output);
    }
}

impl CompositorHandler for CurtainApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl ProvidesRegistryState for CurtainApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

impl ShmHandler for CurtainApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

smithay_client_toolkit::delegate_compositor!(CurtainApp);
smithay_client_toolkit::delegate_output!(CurtainApp);
smithay_client_toolkit::delegate_registry!(CurtainApp);
smithay_client_toolkit::delegate_session_lock!(CurtainApp);
smithay_client_toolkit::delegate_shm!(CurtainApp);
smithay_client_toolkit::reexports::client::delegate_noop!(CurtainApp: ignore wl_buffer::WlBuffer);
