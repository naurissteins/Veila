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
        protocol::{wl_keyboard, wl_output, wl_pointer, wl_surface},
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
    shm::SurfaceBufferPool,
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
    pub(crate) static_overlay: Option<veila_renderer::SoftwareBuffer>,
    pub(crate) static_overlay_revision: u64,
    pub(crate) shm_pool: Option<SurfaceBufferPool>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RenderTimingSample {
    pub(crate) first_frame: bool,
    pub(crate) background_prepare_ms: u64,
    pub(crate) static_overlay_prepare_ms: u64,
    pub(crate) background_restore_ms: u64,
    pub(crate) static_overlay_blend_ms: u64,
    pub(crate) dynamic_overlay_ms: u64,
    pub(crate) shm_pool_prepare_ms: u64,
    pub(crate) commit_ms: u64,
    pub(crate) total_ms: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct StageTimingStats {
    total_ms: u128,
    max_ms: u64,
}

#[derive(Debug, Default)]
pub(crate) struct RenderProfiler {
    frames_rendered: u64,
    first_frames: u64,
    background_prepare: StageTimingStats,
    static_overlay_prepare: StageTimingStats,
    background_restore: StageTimingStats,
    static_overlay_blend: StageTimingStats,
    dynamic_overlay: StageTimingStats,
    shm_pool_prepare: StageTimingStats,
    commit: StageTimingStats,
    total: StageTimingStats,
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
    pub(crate) pointer: Option<wl_pointer::WlPointer>,
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
    pub(crate) render_profiler: RenderProfiler,
}

impl StageTimingStats {
    fn record(&mut self, elapsed_ms: u64) {
        self.total_ms = self.total_ms.saturating_add(u128::from(elapsed_ms));
        self.max_ms = self.max_ms.max(elapsed_ms);
    }

    fn average_ms(self, frames: u64) -> u64 {
        if frames == 0 {
            0
        } else {
            (self.total_ms / u128::from(frames)).min(u128::from(u64::MAX)) as u64
        }
    }
}

impl RenderProfiler {
    pub(crate) fn record(&mut self, sample: RenderTimingSample) {
        self.frames_rendered = self.frames_rendered.saturating_add(1);
        self.first_frames = self
            .first_frames
            .saturating_add(u64::from(sample.first_frame));
        self.background_prepare.record(sample.background_prepare_ms);
        self.static_overlay_prepare
            .record(sample.static_overlay_prepare_ms);
        self.background_restore.record(sample.background_restore_ms);
        self.static_overlay_blend
            .record(sample.static_overlay_blend_ms);
        self.dynamic_overlay.record(sample.dynamic_overlay_ms);
        self.shm_pool_prepare.record(sample.shm_pool_prepare_ms);
        self.commit.record(sample.commit_ms);
        self.total.record(sample.total_ms);
    }

    pub(crate) fn log_summary(&self) {
        if self.frames_rendered == 0 {
            return;
        }

        let average_stages = [
            (
                "background_prepare_ms",
                self.background_prepare.average_ms(self.frames_rendered),
            ),
            (
                "static_overlay_prepare_ms",
                self.static_overlay_prepare.average_ms(self.frames_rendered),
            ),
            (
                "background_restore_ms",
                self.background_restore.average_ms(self.frames_rendered),
            ),
            (
                "static_overlay_blend_ms",
                self.static_overlay_blend.average_ms(self.frames_rendered),
            ),
            (
                "dynamic_overlay_ms",
                self.dynamic_overlay.average_ms(self.frames_rendered),
            ),
            (
                "shm_pool_prepare_ms",
                self.shm_pool_prepare.average_ms(self.frames_rendered),
            ),
            ("commit_ms", self.commit.average_ms(self.frames_rendered)),
        ];
        let max_stages = [
            ("background_prepare_ms", self.background_prepare.max_ms),
            (
                "static_overlay_prepare_ms",
                self.static_overlay_prepare.max_ms,
            ),
            ("background_restore_ms", self.background_restore.max_ms),
            ("static_overlay_blend_ms", self.static_overlay_blend.max_ms),
            ("dynamic_overlay_ms", self.dynamic_overlay.max_ms),
            ("shm_pool_prepare_ms", self.shm_pool_prepare.max_ms),
            ("commit_ms", self.commit.max_ms),
        ];
        let slowest_avg_stage = average_stages
            .iter()
            .max_by_key(|(_, elapsed_ms)| *elapsed_ms)
            .copied()
            .unwrap_or(("total_ms", 0));
        let slowest_max_stage = max_stages
            .iter()
            .max_by_key(|(_, elapsed_ms)| *elapsed_ms)
            .copied()
            .unwrap_or(("total_ms", 0));

        tracing::debug!(
            frames_rendered = self.frames_rendered,
            first_frames = self.first_frames,
            total_avg_ms = self.total.average_ms(self.frames_rendered),
            total_max_ms = self.total.max_ms,
            background_prepare_avg_ms = self.background_prepare.average_ms(self.frames_rendered),
            background_prepare_max_ms = self.background_prepare.max_ms,
            static_overlay_prepare_avg_ms =
                self.static_overlay_prepare.average_ms(self.frames_rendered),
            static_overlay_prepare_max_ms = self.static_overlay_prepare.max_ms,
            background_restore_avg_ms = self.background_restore.average_ms(self.frames_rendered),
            background_restore_max_ms = self.background_restore.max_ms,
            static_overlay_blend_avg_ms =
                self.static_overlay_blend.average_ms(self.frames_rendered),
            static_overlay_blend_max_ms = self.static_overlay_blend.max_ms,
            dynamic_overlay_avg_ms = self.dynamic_overlay.average_ms(self.frames_rendered),
            dynamic_overlay_max_ms = self.dynamic_overlay.max_ms,
            shm_pool_prepare_avg_ms = self.shm_pool_prepare.average_ms(self.frames_rendered),
            shm_pool_prepare_max_ms = self.shm_pool_prepare.max_ms,
            commit_avg_ms = self.commit.average_ms(self.frames_rendered),
            commit_max_ms = self.commit.max_ms,
            slowest_avg_stage = slowest_avg_stage.0,
            slowest_avg_stage_ms = slowest_avg_stage.1,
            slowest_max_stage = slowest_max_stage.0,
            slowest_max_stage_ms = slowest_max_stage.1,
            "curtain render timing summary"
        );
    }
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
        let ui_shell = ShellState::new_with_username(
            theme,
            config.lock.user_hint.clone(),
            config.lock.username.clone(),
            config.lock.avatar_path.clone(),
            config.lock.show_username,
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
            pointer: None,
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
            background: None,
            static_overlay: None,
            static_overlay_revision: 0,
            shm_pool: None,
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

    pub(crate) fn drain_control_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.control_events.try_recv() {
            match event {
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

    pub(crate) fn handle_shell_pointer_press(
        &mut self,
        surface: &wl_surface::WlSurface,
        position: (f64, f64),
        queue_handle: &QueueHandle<Self>,
    ) {
        if self.auth_in_flight {
            return;
        }

        let Some((width, height)) = self
            .lock_surfaces
            .iter()
            .find(|entry| entry.surface.wl_surface() == surface)
            .and_then(|entry| entry.size)
        else {
            return;
        };

        if self
            .ui_shell
            .handle_pointer_press(width as i32, height as i32, position.0, position.1)
        {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_pointer_motion(
        &mut self,
        surface: &wl_surface::WlSurface,
        position: (f64, f64),
        queue_handle: &QueueHandle<Self>,
    ) {
        let Some((width, height)) = self
            .lock_surfaces
            .iter()
            .find(|entry| entry.surface.wl_surface() == surface)
            .and_then(|entry| entry.size)
        else {
            return;
        };

        if self
            .ui_shell
            .handle_pointer_motion(width as i32, height as i32, position.0, position.1)
        {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_pointer_release(
        &mut self,
        surface: &wl_surface::WlSurface,
        position: (f64, f64),
        queue_handle: &QueueHandle<Self>,
    ) {
        if self.auth_in_flight {
            return;
        }

        let Some((width, height)) = self
            .lock_surfaces
            .iter()
            .find(|entry| entry.surface.wl_surface() == surface)
            .and_then(|entry| entry.size)
        else {
            return;
        };

        if self
            .ui_shell
            .handle_pointer_release(width as i32, height as i32, position.0, position.1)
        {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_pointer_leave(&mut self, queue_handle: &QueueHandle<Self>) {
        if self.ui_shell.handle_pointer_leave() {
            self.render_all_surfaces(queue_handle);
        }
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
            .map(|color| ClearColor::rgba(color.0, color.1, color.2, color.3)),
        tint_opacity: config.tint_opacity,
    }
}
