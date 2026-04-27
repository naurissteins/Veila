mod interaction;
mod profiler;

use std::{
    path::{Path, PathBuf},
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
        protocol::{wl_keyboard, wl_output, wl_surface},
    },
    registry::RegistryState,
    seat::{SeatState, pointer::ThemedPointer},
    session_lock::{SessionLock, SessionLockState, SessionLockSurface},
    shm::Shm,
};
use veila_common::{
    AppConfig, BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot,
    config::{BackgroundConfig, BackgroundOutputConfig},
};
use veila_renderer::{
    ClearColor,
    background::{BackgroundAsset, BackgroundGradient, BackgroundTreatment},
    shm::SurfaceBufferPool,
};
use veila_ui::{ShellState, ShellTheme};

use crate::{
    CurtainOptions,
    background::BackgroundEvent,
    ipc::auth::AuthEvent,
    ipc::control::{ControlEvent, spawn_listener},
};

pub(crate) use profiler::{RenderProfiler, RenderTimingSample};

pub(crate) struct ManagedLockSurface {
    pub(crate) output: wl_output::WlOutput,
    pub(crate) surface: SessionLockSurface,
    pub(crate) size: Option<(u32, u32)>,
    pub(crate) background_path: Option<PathBuf>,
    pub(crate) background: Option<veila_renderer::SoftwareBuffer>,
    pub(crate) scene_base: Option<veila_renderer::SoftwareBuffer>,
    pub(crate) scene_base_revision: u64,
    pub(crate) static_overlay: Option<veila_renderer::SoftwareBuffer>,
    pub(crate) static_overlay_revision: u64,
    pub(crate) shm_pool: Option<SurfaceBufferPool>,
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
    pub(crate) pointer: Option<ThemedPointer>,
    pub(crate) lock_surfaces: Vec<ManagedLockSurface>,
    pub(crate) notify_socket: Option<PathBuf>,
    daemon_socket: Option<PathBuf>,
    control_socket: Option<PathBuf>,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) background_path: Option<PathBuf>,
    pub(crate) background_outputs: Vec<BackgroundOutputConfig>,
    auth_events: Receiver<AuthEvent>,
    auth_sender: Sender<AuthEvent>,
    pub(crate) background_sender: Sender<BackgroundEvent>,
    pub(crate) background_events: Receiver<BackgroundEvent>,
    control_events: Receiver<ControlEvent>,
    pub(crate) background_asset: BackgroundAsset,
    pub(crate) background_gradient: Option<BackgroundGradient>,
    pub(crate) background_treatment: BackgroundTreatment,
    pub(crate) background_color: ClearColor,
    pub(crate) weather_snapshot: Option<WeatherSnapshot>,
    pub(crate) battery_snapshot: Option<BatterySnapshot>,
    pub(crate) now_playing_snapshot: Option<NowPlayingSnapshot>,
    pub(crate) ui_shell: ShellState,
    pub(crate) lock_wait_timeout: Duration,
    pub(crate) startup_started_at: Instant,
    lock_started_at: Instant,
    lock_acquisition_started: bool,
    pub(crate) session_locked: bool,
    pub(crate) session_locked_at: Option<Instant>,
    pub(crate) session_finished: bool,
    pub(crate) exit_requested: bool,
    pub(crate) ready_notified: bool,
    pub(crate) first_surface_configured_logged: bool,
    pub(crate) all_surfaces_configured_logged: bool,
    pub(crate) background_render_started: bool,
    auth_in_flight: bool,
    next_auth_attempt_id: u64,
    pub(crate) has_keyboard_focus: bool,
    pub(crate) keyboard_layout_labels: Vec<String>,
    pub(crate) active_keyboard_layout: u32,
    pub(crate) failure_reason: Option<String>,
    pub(crate) render_profiler: RenderProfiler,
}

impl CurtainApp {
    pub(crate) fn new(
        connection: Connection,
        globals: &GlobalList,
        queue_handle: &QueueHandle<Self>,
        options: CurtainOptions,
        startup_started_at: Instant,
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
            background_gradient(&config.background),
            background_treatment(&config.background),
        )
        .context("failed to prepare fallback background")?;
        let background_gradient = background_gradient(&config.background);
        let background_treatment = background_treatment(&config.background);
        let ui_shell = ShellState::new_with_username_and_widgets(
            theme,
            config.lock.user_hint.clone(),
            config.lock.username.clone(),
            config.lock.avatar_path.clone(),
            config.lock.show_username,
            config.weather.normalized_location(),
            options.weather_snapshot.clone(),
            config.weather.unit,
            options.battery_snapshot.clone(),
            options.now_playing_snapshot.clone(),
        );
        let lock_wait_timeout = Duration::from_secs(config.lock.acquire_timeout_seconds.max(1));

        tracing::info!(
            config = loaded_config
                .path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "defaults".to_string()),
            background_mode = config.background.effective_mode().as_str(),
            background_image = config
                .background
                .resolved_path()
                .as_deref()
                .map(|path| path.display().to_string()),
            background_output_overrides = config.background.outputs.len(),
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
            pointer: None,
            lock_surfaces: Vec::new(),
            notify_socket: options.notify_socket,
            daemon_socket: options.daemon_socket,
            control_socket: options.control_socket,
            config_path: options.config_path,
            background_path: config.background.resolved_path(),
            background_outputs: config.background.outputs.clone(),
            auth_events,
            auth_sender,
            background_sender,
            background_events,
            control_events,
            background_asset,
            background_gradient,
            background_treatment,
            background_color,
            weather_snapshot: options.weather_snapshot,
            battery_snapshot: options.battery_snapshot,
            now_playing_snapshot: options.now_playing_snapshot,
            ui_shell,
            lock_wait_timeout,
            startup_started_at,
            lock_started_at: Instant::now(),
            session_locked: false,
            session_locked_at: None,
            session_finished: false,
            exit_requested: false,
            ready_notified: false,
            first_surface_configured_logged: false,
            all_surfaces_configured_logged: false,
            background_render_started: false,
            auth_in_flight: false,
            next_auth_attempt_id: 1,
            has_keyboard_focus: false,
            keyboard_layout_labels: Vec::new(),
            active_keyboard_layout: 0,
            failure_reason: None,
            render_profiler: RenderProfiler::default(),
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
            background_path: None,
            background: None,
            scene_base: None,
            scene_base_revision: 0,
            static_overlay: None,
            static_overlay_revision: 0,
            shm_pool: None,
        });
        self.background_render_started = false;

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
        self.render_profiler.log_summary();

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

    pub(crate) fn surface_has_focus_target(&self, surface: &wl_surface::WlSurface) -> bool {
        self.lock_surfaces
            .iter()
            .any(|entry| entry.surface.wl_surface() == surface)
    }

    pub(crate) fn background_path_for_surface(&self, index: usize) -> Option<&Path> {
        let output_name = self
            .output_state
            .info(&self.lock_surfaces[index].output)
            .and_then(|info| info.name.clone());

        output_name
            .as_deref()
            .and_then(|name| {
                self.background_outputs
                    .iter()
                    .find(|output| output.name == name)
            })
            .map(|output| output.path.as_path())
            .or(self.background_path.as_deref())
    }
}

pub(crate) fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn elapsed_us(started_at: Instant) -> u64 {
    started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn background_treatment(
    config: &veila_common::config::BackgroundConfig,
) -> BackgroundTreatment {
    BackgroundTreatment {
        blur_radius: config.blur_radius,
        dim_strength: config.dim_strength,
        tint: config
            .tint
            .map(|color| ClearColor::rgba(color.0, color.1, color.2, color.3)),
        tint_opacity: config.tint_opacity,
    }
}

pub(crate) fn background_gradient(config: &BackgroundConfig) -> Option<BackgroundGradient> {
    let gradient = config.resolved_gradient()?;

    Some(BackgroundGradient {
        top_left: to_background_color(gradient.top_left),
        top_right: to_background_color(gradient.top_right),
        bottom_left: to_background_color(gradient.bottom_left),
        bottom_right: to_background_color(gradient.bottom_right),
    })
}

fn to_background_color(color: veila_common::RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}
