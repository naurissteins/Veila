mod interaction;
mod memory;
mod power;
mod profiler;
mod repeat;
mod resume;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
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
    registry::{GlobalProxy, RegistryState},
    seat::{SeatState, pointer::ThemedPointer},
    session_lock::{SessionLock, SessionLockState, SessionLockSurface},
    shm::Shm,
};
use veila_common::{
    AppConfig, BatterySnapshot, NowPlayingSnapshot, OutputUiMode, WeatherSnapshot,
    config::{
        BackgroundConfig, BackgroundLayeredBaseMode, BackgroundLayeredConfig,
        BackgroundOutputConfig, BackgroundScaling as ConfigBackgroundScaling,
    },
    ipc::LockPowerStatusSnapshot,
};
use veila_renderer::{
    ClearColor,
    background::{
        BackgroundAsset, BackgroundGradient, BackgroundLayered, BackgroundLayeredBase,
        BackgroundLayeredBlob, BackgroundRadial, BackgroundScaling, BackgroundTreatment,
        GeneratedBackground,
    },
    shm::SurfaceBufferPool,
};
use veila_ui::{ShellState, ShellTheme};
use wayland_protocols_wlr::output_power_management::v1::client::{
    zwlr_output_power_manager_v1, zwlr_output_power_v1,
};

use crate::{
    CurtainOptions,
    background::{BackgroundEvent, BackgroundSlideshow},
    ipc::auth::AuthEvent,
    ipc::control::{ControlEvent, spawn_listener},
};

pub(crate) use power::ScreenOffState;
pub(crate) use profiler::{RenderProfiler, RenderTimingSample};
pub(crate) use repeat::KeyRepeatState;
pub(crate) use resume::ResumeInputState;

pub(crate) struct ScratchBuffer {
    pub(crate) buffer: veila_renderer::SoftwareBuffer,
    pub(crate) final_frame_key: Option<FinalFrameKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FinalFrameKey {
    pub(crate) scene_base_ptr: usize,
    pub(crate) frame_size: veila_renderer::FrameSize,
    pub(crate) scene_revision: u64,
    pub(crate) render_scale: u32,
}

pub(crate) struct ManagedLockSurface {
    pub(crate) output: wl_output::WlOutput,
    pub(crate) surface: SessionLockSurface,
    pub(crate) size: Option<SurfaceSize>,
    pub(crate) background_path: Option<PathBuf>,
    pub(crate) background: Option<veila_renderer::SoftwareBuffer>,
    pub(crate) scene_base: Option<Arc<veila_renderer::SoftwareBuffer>>,
    pub(crate) scene_base_revision: u64,
    pub(crate) shm_pool: Option<SurfaceBufferPool>,
    pub(crate) output_power: Option<zwlr_output_power_v1::ZwlrOutputPowerV1>,
    pub(crate) preferred_scale: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurfaceSize {
    pub(crate) logical_width: u32,
    pub(crate) logical_height: u32,
    pub(crate) buffer: veila_renderer::FrameSize,
    pub(crate) scale: i32,
}

impl SurfaceSize {
    pub(crate) fn new(logical_width: u32, logical_height: u32, scale: i32) -> Self {
        let scale = scale.max(1) as u32;
        Self {
            logical_width,
            logical_height,
            buffer: veila_renderer::FrameSize::new(
                logical_width.saturating_mul(scale),
                logical_height.saturating_mul(scale),
            ),
            scale: scale as i32,
        }
    }
}

pub(crate) struct CurtainApp {
    pub(crate) connection: Connection,
    pub(crate) compositor_state: CompositorState,
    pub(crate) output_state: OutputState,
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) session_lock_state: SessionLockState,
    pub(crate) output_power_manager:
        GlobalProxy<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>,
    pub(crate) session_lock: Option<SessionLock>,
    pub(crate) shm: Shm,
    pub(crate) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(crate) pointer: Option<ThemedPointer>,
    pub(crate) lock_surfaces: Vec<ManagedLockSurface>,
    pub(crate) scratch_buffers: Vec<ScratchBuffer>,
    pub(crate) notify_socket: Option<PathBuf>,
    daemon_socket: Option<PathBuf>,
    control_socket: Option<PathBuf>,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) background_path: Option<PathBuf>,
    pub(crate) background_outputs: Vec<BackgroundOutputConfig>,
    pub(crate) slideshow: Option<BackgroundSlideshow>,
    auth_events: Receiver<AuthEvent>,
    auth_sender: Sender<AuthEvent>,
    pub(crate) background_sender: Sender<BackgroundEvent>,
    pub(crate) background_events: Receiver<BackgroundEvent>,
    control_events: Receiver<ControlEvent>,
    pub(crate) background_asset: BackgroundAsset,
    pub(crate) background_generated: Option<GeneratedBackground>,
    pub(crate) background_treatment: BackgroundTreatment,
    pub(crate) background_color: ClearColor,
    pub(crate) ui_output_mode: OutputUiMode,
    pub(crate) ui_output_name: Option<String>,
    pub(crate) weather_snapshot: Option<WeatherSnapshot>,
    pub(crate) battery_snapshot: Option<BatterySnapshot>,
    pub(crate) now_playing_snapshot: Option<NowPlayingSnapshot>,
    pub(crate) remote_power_status: Option<LockPowerStatusSnapshot>,
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
    pub(crate) first_surface_configured_at: Option<Instant>,
    pub(crate) all_surfaces_configured_logged: bool,
    pub(crate) all_surfaces_configured_at: Option<Instant>,
    pub(crate) background_render_started: bool,
    auth_in_flight: bool,
    next_auth_attempt_id: u64,
    pub(crate) has_keyboard_focus: bool,
    pub(crate) ctrl_active: bool,
    pub(crate) keyboard_layout_labels: Vec<String>,
    pub(crate) active_keyboard_layout: u32,
    pub(crate) failure_reason: Option<String>,
    pub(crate) render_profiler: RenderProfiler,
    pub(crate) backspace_repeat: Option<KeyRepeatState>,
    pub(crate) screen_off: ScreenOffState,
    pub(crate) resume_input: ResumeInputState,
    pub(crate) wake_key_release_pending: bool,
    pub(crate) wake_pointer_release_pending: bool,
    pub(crate) post_ready_nonfirst_renders: u32,
    pub(crate) post_ready_memory_logged: bool,
}

impl CurtainApp {
    pub(crate) fn daemon_socket_path(&self) -> Option<PathBuf> {
        self.daemon_socket.clone()
    }

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
            background_generated(&config.background),
            background_treatment(&config.background),
        )
        .context("failed to prepare fallback background")?;
        let background_generated = background_generated(&config.background);
        let background_treatment = background_treatment(&config.background);
        let slideshow = BackgroundSlideshow::load(
            &config.background,
            options.initial_background_path.as_deref(),
        );
        let background_path = slideshow
            .as_ref()
            .map(|slideshow| slideshow.current_path().to_path_buf())
            .or_else(|| config.background.resolved_path());
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
        let screen_off_delay = config
            .lock
            .screen_off_seconds
            .filter(|seconds| *seconds > 0)
            .map(Duration::from_secs);
        let output_power_manager = GlobalProxy::from(globals.bind(queue_handle, 1..=1, ()));

        if screen_off_delay.is_some() && output_power_manager.get().is_err() {
            tracing::warn!(
                screen_off_seconds = config.lock.screen_off_seconds,
                "output power management is unavailable; locked screen-off timer is disabled"
            );
        }

        tracing::info!(
            config = loaded_config
                .path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "defaults".to_string()),
            background_mode = config.background.effective_mode().as_str(),
            background_image = background_path
                .as_deref()
                .map(|path| path.display().to_string()),
            background_output_overrides = config.background.outputs.len(),
            background_slideshow_images = slideshow.as_ref().map(BackgroundSlideshow::len),
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
            output_power_manager,
            session_lock: None,
            shm: Shm::bind(globals, queue_handle)
                .context("compositor does not advertise wl_shm")?,
            keyboard: None,
            pointer: None,
            lock_surfaces: Vec::new(),
            scratch_buffers: Vec::new(),
            notify_socket: options.notify_socket,
            daemon_socket: options.daemon_socket,
            control_socket: options.control_socket,
            config_path: options.config_path,
            background_path,
            background_outputs: config.background.outputs.clone(),
            slideshow,
            auth_events,
            auth_sender,
            background_sender,
            background_events,
            control_events,
            background_asset,
            background_generated,
            background_treatment,
            background_color,
            ui_output_mode: config.visuals.output_ui_mode(),
            ui_output_name: config.visuals.ui_output_name().map(str::to_owned),
            weather_snapshot: options.weather_snapshot,
            battery_snapshot: options.battery_snapshot,
            now_playing_snapshot: options.now_playing_snapshot,
            remote_power_status: None,
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
            first_surface_configured_at: None,
            all_surfaces_configured_logged: false,
            all_surfaces_configured_at: None,
            background_render_started: false,
            auth_in_flight: false,
            next_auth_attempt_id: 1,
            has_keyboard_focus: false,
            ctrl_active: false,
            keyboard_layout_labels: Vec::new(),
            active_keyboard_layout: 0,
            failure_reason: None,
            render_profiler: RenderProfiler::default(),
            backspace_repeat: None,
            screen_off: ScreenOffState::new(screen_off_delay),
            resume_input: ResumeInputState::new(),
            wake_key_release_pending: false,
            wake_pointer_release_pending: false,
            post_ready_nonfirst_renders: 0,
            post_ready_memory_logged: false,
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

        let output_power = self.bind_output_power_for_surface(&output, queue_handle);
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
            shm_pool: None,
            output_power,
            preferred_scale: 1,
        });
        self.background_render_started = false;

        if self.outputs_powered_off()
            && let Some(output_power) = self
                .lock_surfaces
                .last()
                .and_then(|entry| entry.output_power.as_ref())
        {
            output_power.set_mode(zwlr_output_power_v1::Mode::Off);
        }

        Ok(())
    }

    pub(crate) fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    pub(crate) fn can_stop(&self) -> bool {
        self.failure_reason.is_some()
            || (self.exit_requested && (self.session_locked || self.session_finished))
    }

    pub(crate) fn animation_poll_interval(&self) -> Duration {
        let shell_interval = self.ui_shell.animation_poll_interval();
        let now = Instant::now();
        let repeat_interval = self
            .backspace_repeat
            .as_ref()
            .map(|backspace_repeat| backspace_repeat.due_in(now))
            .unwrap_or(shell_interval);
        let slideshow_interval = self
            .slideshow
            .as_ref()
            .and_then(|slideshow| slideshow.next_due_in(now))
            .unwrap_or(shell_interval);
        let screen_off_interval = self
            .screen_off
            .due_in(now, self.session_locked)
            .unwrap_or(shell_interval);
        let power_status_interval = self
            .power_status_poll_interval(now)
            .unwrap_or(shell_interval);

        shell_interval
            .min(repeat_interval)
            .min(slideshow_interval)
            .min(screen_off_interval)
            .min(power_status_interval)
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

        if self.outputs_powered_off() {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
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
        if self.slideshow.is_some() {
            return self.background_path.as_deref();
        }

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

    pub(crate) fn ui_visible_on_surface(&self, index: usize) -> bool {
        match self.ui_output_mode {
            OutputUiMode::All => true,
            OutputUiMode::Single => self.selected_ui_surface_index() == Some(index),
        }
    }

    fn selected_ui_surface_index(&self) -> Option<usize> {
        if let Some(selected_name) = self.ui_output_name.as_deref()
            && let Some(index) = self.lock_surfaces.iter().position(|surface| {
                self.output_state
                    .info(&surface.output)
                    .and_then(|info| info.name.clone())
                    .as_deref()
                    == Some(selected_name)
            })
        {
            return Some(index);
        }

        self.lock_surfaces
            .iter()
            .position(|surface| surface.size.is_some())
            .or_else(|| (!self.lock_surfaces.is_empty()).then_some(0))
    }
}

pub(crate) fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn elapsed_us(started_at: Instant) -> u64 {
    started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn duration_ms_between(started_at: Option<Instant>, ended_at: Instant) -> Option<u64> {
    started_at.map(|started_at| {
        ended_at
            .saturating_duration_since(started_at)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    })
}

pub(crate) fn background_treatment(
    config: &veila_common::config::BackgroundConfig,
) -> BackgroundTreatment {
    BackgroundTreatment {
        blur_radius: config.blur_strength,
        dim_strength: config.dim_strength,
        tint: config
            .tint
            .map(|color| ClearColor::rgba(color.0, color.1, color.2, color.3)),
        scaling: to_background_scaling(config.scaling),
    }
}

fn to_background_scaling(scaling: ConfigBackgroundScaling) -> BackgroundScaling {
    match scaling {
        ConfigBackgroundScaling::Fill => BackgroundScaling::Fill,
        ConfigBackgroundScaling::Fit => BackgroundScaling::Fit,
        ConfigBackgroundScaling::Center => BackgroundScaling::Center,
        ConfigBackgroundScaling::Tile => BackgroundScaling::Tile,
        ConfigBackgroundScaling::Stretch => BackgroundScaling::Stretch,
    }
}

pub(crate) fn background_generated(config: &BackgroundConfig) -> Option<GeneratedBackground> {
    if let Some(gradient) = config.resolved_gradient() {
        return Some(GeneratedBackground::Gradient(BackgroundGradient {
            top_left: to_background_color(gradient.top_left),
            top_right: to_background_color(gradient.top_right),
            bottom_left: to_background_color(gradient.bottom_left),
            bottom_right: to_background_color(gradient.bottom_right),
        }));
    }

    if let Some(radial) = config.resolved_radial() {
        return Some(GeneratedBackground::Radial(BackgroundRadial {
            center: to_background_color(radial.center),
            edge: to_background_color(radial.edge),
            center_x: radial.center_x,
            center_y: radial.center_y,
            radius: radial.radius,
        }));
    }

    config
        .resolved_layered()
        .map(|layered| GeneratedBackground::Layered(to_layered_background(&layered)))
}

fn to_background_color(color: veila_common::RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

fn to_layered_background(config: &BackgroundLayeredConfig) -> BackgroundLayered {
    let base = match config.base.effective_mode() {
        BackgroundLayeredBaseMode::Gradient => {
            let gradient = config.base.gradient.clone().unwrap_or_default();
            BackgroundLayeredBase::Gradient(BackgroundGradient {
                top_left: to_background_color(gradient.top_left),
                top_right: to_background_color(gradient.top_right),
                bottom_left: to_background_color(gradient.bottom_left),
                bottom_right: to_background_color(gradient.bottom_right),
            })
        }
        BackgroundLayeredBaseMode::Radial => {
            let radial = config.base.radial.clone().unwrap_or_default();
            BackgroundLayeredBase::Radial(BackgroundRadial {
                center: to_background_color(radial.center),
                edge: to_background_color(radial.edge),
                center_x: radial.center_x,
                center_y: radial.center_y,
                radius: radial.radius,
            })
        }
        BackgroundLayeredBaseMode::Solid => {
            BackgroundLayeredBase::Solid(to_background_color(config.base.color))
        }
    };

    let mut blobs = [None; 3];
    for (slot, blob) in blobs.iter_mut().zip(config.blobs.iter().take(3)) {
        *slot = Some(BackgroundLayeredBlob {
            color: blob_color(blob.color, blob.opacity),
            x: blob.x,
            y: blob.y,
            size: blob.size,
        });
    }

    BackgroundLayered { base, blobs }
}

fn blob_color(color: veila_common::RgbColor, opacity: u8) -> ClearColor {
    let alpha = ((u16::from(color.3) * u16::from(opacity.min(100)) + 50) / 100) as u8;
    ClearColor::rgba(color.0, color.1, color.2, alpha)
}

#[cfg(test)]
mod tests {
    use super::SurfaceSize;

    #[test]
    fn surface_size_tracks_logical_and_scaled_buffer_size() {
        let size = SurfaceSize::new(1920, 1080, 2);

        assert_eq!(size.logical_width, 1920);
        assert_eq!(size.logical_height, 1080);
        assert_eq!(size.buffer.width, 3840);
        assert_eq!(size.buffer.height, 2160);
        assert_eq!(size.scale, 2);
    }

    #[test]
    fn surface_size_never_uses_zero_or_negative_scale() {
        let size = SurfaceSize::new(800, 600, 0);

        assert_eq!(size.buffer.width, 800);
        assert_eq!(size.buffer.height, 600);
        assert_eq!(size.scale, 1);
    }
}
