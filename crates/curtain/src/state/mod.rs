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
    reexports::{
        client::{
            Connection, QueueHandle,
            globals::GlobalList,
            protocol::{wl_keyboard, wl_output, wl_surface},
        },
        protocols::wp::{
            fractional_scale::v1::client::{
                wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
            },
            viewporter::client::{wp_viewport, wp_viewporter},
        },
    },
    registry::{GlobalProxy, RegistryState},
    seat::{SeatState, pointer::ThemedPointer},
    session_lock::{SessionLock, SessionLockState, SessionLockSurface},
    shm::Shm,
};
use veila_common::{
    AppConfig, BatterySnapshot, LoadedConfig, NowPlayingSnapshot, OutputUiMode, WeatherSnapshot,
    config::{
        BackgroundConfig, BackgroundLayeredBaseMode, BackgroundLayeredConfig,
        BackgroundOutputConfig, BackgroundScaling as ConfigBackgroundScaling,
    },
    ipc::{CurtainLatencyReport, LatencyReportMode, LockPowerStatusSnapshot},
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
    keyboard_cache::load_keyboard_layout_label,
};

pub(crate) use power::ScreenOffState;
pub(crate) use profiler::{DirtyRenderTimingSample, RenderProfiler, RenderTimingSample};
pub(crate) use repeat::KeyRepeatState;
pub(crate) use resume::ResumeInputState;

const EMERGENCY_BACKGROUND: ClearColor = ClearColor::opaque(12, 14, 18);

pub(crate) struct ManagedLockSurface {
    pub(crate) output: wl_output::WlOutput,
    pub(crate) surface: SessionLockSurface,
    pub(crate) size: Option<SurfaceSize>,
    pub(crate) background_path: Option<PathBuf>,
    pub(crate) background: Option<veila_renderer::SoftwareBuffer>,
    pub(crate) scene_base: Option<Arc<veila_renderer::SoftwareBuffer>>,
    pub(crate) scene_base_revision: u64,
    pub(crate) scene_base_has_layers: bool,
    pub(crate) shm_pool: Option<SurfaceBufferPool>,
    pub(crate) output_power: Option<zwlr_output_power_v1::ZwlrOutputPowerV1>,
    pub(crate) preferred_scale: i32,
    pub(crate) preferred_fractional_scale: Option<u32>,
    pub(crate) fractional_scale: Option<wp_fractional_scale_v1::WpFractionalScaleV1>,
    pub(crate) viewport: Option<wp_viewport::WpViewport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputRole {
    PrimaryPrompt,
    SecondaryCurtain,
}

impl OutputRole {
    pub(crate) fn renders_shell(self) -> bool {
        matches!(self, Self::PrimaryPrompt)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PrimaryPrompt => "primary_prompt",
            Self::SecondaryCurtain => "secondary_curtain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurfaceSize {
    pub(crate) logical_width: u32,
    pub(crate) logical_height: u32,
    pub(crate) buffer: veila_renderer::FrameSize,
    pub(crate) scale: i32,
    pub(crate) fractional_scale: Option<u32>,
}

impl SurfaceSize {
    pub(crate) fn new_with_fractional_scale(
        logical_width: u32,
        logical_height: u32,
        scale: i32,
        fractional_scale: Option<u32>,
    ) -> Self {
        let scale = scale.max(1) as u32;
        Self {
            logical_width,
            logical_height,
            buffer: veila_renderer::FrameSize::new(
                logical_width.saturating_mul(scale),
                logical_height.saturating_mul(scale),
            ),
            scale: scale as i32,
            fractional_scale,
        }
    }

    pub(crate) fn buffer_scale_for_commit(self) -> i32 {
        if self.fractional_scale.is_some() {
            1
        } else {
            self.scale.max(1)
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
    pub(crate) fractional_scale_manager:
        GlobalProxy<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    pub(crate) viewporter: GlobalProxy<wp_viewporter::WpViewporter>,
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
    pub(crate) hide_cursor: bool,
    pub(crate) allow_empty_password: bool,
    pub(crate) power_off_secondary_outputs: bool,
    pub(crate) secondary_outputs_powered_off: bool,
    pub(crate) weather_snapshot: Option<WeatherSnapshot>,
    pub(crate) battery_snapshot: Option<BatterySnapshot>,
    pub(crate) now_playing_snapshot: Option<NowPlayingSnapshot>,
    pub(crate) remote_power_status: Option<LockPowerStatusSnapshot>,
    pub(crate) ui_shell: ShellState,
    pub(crate) avatar_path: Option<PathBuf>,
    pub(crate) avatar_load_started: bool,
    pub(crate) lock_wait_timeout: Duration,
    pub(crate) startup_started_at: Instant,
    lock_started_at: Instant,
    lock_acquisition_started: bool,
    pub(crate) session_locked: bool,
    pub(crate) session_locked_at: Option<Instant>,
    pub(crate) session_finished: bool,
    pub(crate) exit_requested: bool,
    unlock_authorized: bool,
    pub(crate) ready_notified: bool,
    pub(crate) latency_report: LatencyReportMode,
    pub(crate) latency_timings: CurtainLatencyReport,
    pub(crate) first_surface_configured_logged: bool,
    pub(crate) first_surface_configured_at: Option<Instant>,
    pub(crate) all_surfaces_configured_logged: bool,
    pub(crate) all_surfaces_configured_at: Option<Instant>,
    pub(crate) background_render_started: bool,
    auth_in_flight: bool,
    auth_accepted_at: Option<Instant>,
    next_auth_attempt_id: u64,
    pub(crate) has_keyboard_focus: bool,
    pub(crate) focused_surface_index: Option<usize>,
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
    pub(crate) pending_pre_ready_redraw: bool,
    pub(crate) first_frame_committed_at: Option<Instant>,
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
        let force_emergency_ui = options.force_emergency_ui;
        let mut emergency_reason = None;
        let loaded_config = match AppConfig::load(options.config_path.as_deref()) {
            Ok(loaded_config) => loaded_config,
            Err(error) => {
                let reason = format!("failed to load curtain config: {error:#}");
                tracing::warn!("{reason}; emergency fallback UI active");
                emergency_reason = Some(reason);
                LoadedConfig {
                    path: options.config_path.clone(),
                    config: AppConfig::default(),
                }
            }
        };
        let config = loaded_config.config;
        let emergency_active = force_emergency_ui || emergency_reason.is_some();
        let theme = ShellTheme::from_config(&config);
        let background_color = if emergency_active {
            EMERGENCY_BACKGROUND
        } else {
            theme.background
        };
        let background_asset = BackgroundAsset::load(
            None,
            background_color,
            (!emergency_active)
                .then(|| background_generated(&config.background))
                .flatten(),
            if emergency_active {
                BackgroundTreatment::default()
            } else {
                background_treatment(&config.background)
            },
        )
        .context("failed to prepare fallback background")?;
        let background_generated = (!emergency_active)
            .then(|| background_generated(&config.background))
            .flatten();
        let background_treatment = if emergency_active {
            BackgroundTreatment::default()
        } else {
            background_treatment(&config.background)
        };
        let slideshow = (!emergency_active)
            .then(|| {
                BackgroundSlideshow::load(
                    &config.background,
                    options.initial_background_path.as_deref(),
                )
            })
            .flatten();
        let background_path = if emergency_active {
            None
        } else {
            slideshow
                .as_ref()
                .map(|slideshow| slideshow.current_path().to_path_buf())
                .or_else(|| config.background.resolved_path())
        };
        let avatar_path = config.avatar_image_path().map(std::path::Path::to_path_buf);
        let cached_avatar = veila_ui::load_cached_avatar(avatar_path.clone());
        let weather_location = effective_weather_location(&config);
        let weather_snapshot =
            effective_weather_snapshot(&config, options.weather_snapshot.clone());
        let battery_snapshot =
            effective_battery_snapshot(&config, options.battery_snapshot.clone());
        let mut ui_shell = ShellState::new_with_avatar_and_widgets(
            theme,
            Some(config.visuals.input_placeholder()),
            config.visuals.username_text().map(str::to_owned),
            config.visuals.username_enabled(),
            weather_location,
            weather_snapshot,
            config.weather.unit,
            battery_snapshot,
            options.now_playing_snapshot.clone(),
            cached_avatar,
        );
        if ui_shell.keyboard_enabled() {
            ui_shell.set_keyboard_layout_label(load_keyboard_layout_label());
        }
        if emergency_active {
            if force_emergency_ui {
                tracing::info!("emergency fallback UI forced by command line");
            }
            if let Some(reason) = emergency_reason.as_deref() {
                tracing::warn!(reason, "emergency fallback UI active");
            }
            ui_shell.activate_emergency();
        }
        let lock_wait_timeout = Duration::from_secs(config.lock.acquire_timeout_seconds.max(1));
        let screen_off_delay = config
            .lock
            .screen_off_seconds
            .filter(|seconds| *seconds > 0)
            .map(Duration::from_secs);
        let power_off_secondary_outputs =
            !emergency_active && config.lock.power_off_secondary_outputs;
        let output_power_manager = GlobalProxy::from(globals.bind(queue_handle, 1..=1, ()));
        let fractional_scale_manager = GlobalProxy::from(globals.bind(queue_handle, 1..=1, ()));
        let viewporter = GlobalProxy::from(globals.bind(queue_handle, 1..=1, ()));

        if (screen_off_delay.is_some() || power_off_secondary_outputs)
            && output_power_manager.get().is_err()
        {
            tracing::warn!(
                screen_off_seconds = config.lock.screen_off_seconds,
                power_off_secondary_outputs,
                "output power management is unavailable; locked output power features are disabled"
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
            fractional_scale_manager,
            viewporter,
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
            background_path,
            background_outputs: if emergency_active {
                Vec::new()
            } else {
                config.background.outputs.clone()
            },
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
            ui_output_mode: if emergency_active {
                OutputUiMode::All
            } else {
                config.visuals.output_ui_mode()
            },
            ui_output_name: if emergency_active {
                None
            } else {
                config.visuals.ui_output_name().map(str::to_owned)
            },
            hide_cursor: config.lock.hide_cursor,
            allow_empty_password: config.lock.allow_empty_password,
            power_off_secondary_outputs,
            secondary_outputs_powered_off: false,
            weather_snapshot: options.weather_snapshot,
            battery_snapshot: options.battery_snapshot,
            now_playing_snapshot: options.now_playing_snapshot,
            remote_power_status: None,
            ui_shell,
            avatar_path,
            avatar_load_started: false,
            lock_wait_timeout,
            startup_started_at,
            lock_started_at: Instant::now(),
            session_locked: false,
            session_locked_at: None,
            session_finished: false,
            exit_requested: false,
            unlock_authorized: false,
            ready_notified: false,
            latency_report: options.latency_report,
            latency_timings: CurtainLatencyReport::default(),
            first_surface_configured_logged: false,
            first_surface_configured_at: None,
            all_surfaces_configured_logged: false,
            all_surfaces_configured_at: None,
            background_render_started: false,
            auth_in_flight: false,
            auth_accepted_at: None,
            next_auth_attempt_id: 1,
            has_keyboard_focus: false,
            focused_surface_index: None,
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
            pending_pre_ready_redraw: false,
            first_frame_committed_at: None,
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
        let viewport = self
            .viewporter
            .get()
            .ok()
            .map(|viewporter| viewporter.get_viewport(&wl_surface, queue_handle, ()));
        let fractional_scale = match (self.fractional_scale_manager.get(), viewport.is_some()) {
            (Ok(manager), true) => {
                Some(manager.get_fractional_scale(&wl_surface, queue_handle, wl_surface.clone()))
            }
            _ => None,
        };
        let surface = session_lock.create_lock_surface(wl_surface, &output, queue_handle);
        self.lock_surfaces.push(ManagedLockSurface {
            output,
            surface,
            size: None,
            background_path: None,
            background: None,
            scene_base: None,
            scene_base_revision: 0,
            scene_base_has_layers: false,
            shm_pool: None,
            output_power,
            preferred_scale: 1,
            preferred_fractional_scale: None,
            fractional_scale,
            viewport,
        });
        self.background_render_started = false;

        if self.outputs_powered_off()
            && let Some(output_power) = self
                .lock_surfaces
                .last()
                .and_then(|entry| entry.output_power.as_ref())
        {
            output_power.set_mode(zwlr_output_power_v1::Mode::Off);
        } else if self.secondary_outputs_powered_off {
            let index = self.lock_surfaces.len().saturating_sub(1);
            if self.output_role_for_surface(index) == OutputRole::SecondaryCurtain
                && let Some(output_power) = self.lock_surfaces[index].output_power.as_ref()
            {
                output_power.set_mode(zwlr_output_power_v1::Mode::Off);
            }
        }

        Ok(())
    }

    pub(crate) fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Records that daemon authorized this unlock
    pub(crate) fn authorize_unlock(&mut self) {
        self.unlock_authorized = true;
    }

    pub(crate) fn request_exit_from_signal(&mut self) {
        // Standalone --lock sessions have no daemon to authenticate against, so a signal is the
        // only way out. Daemon-managed locks must still wait for an authorized unlock
        if self.control_socket.is_none() {
            self.authorize_unlock();
        }
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

    pub(crate) fn activate_emergency_ui(&mut self, reason: &str) -> Result<()> {
        if self.ui_shell.emergency_active() {
            return Ok(());
        }

        tracing::warn!(reason, "switching to emergency fallback UI");
        self.ui_shell.activate_emergency();
        self.background_path = None;
        self.background_outputs.clear();
        self.slideshow = None;
        self.background_generated = None;
        self.background_treatment = BackgroundTreatment::default();
        self.background_color = EMERGENCY_BACKGROUND;
        self.background_asset = BackgroundAsset::load(
            None,
            EMERGENCY_BACKGROUND,
            None,
            BackgroundTreatment::default(),
        )
        .context("failed to prepare emergency fallback background")?;
        if self.secondary_outputs_powered_off {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
        }
        self.ui_output_mode = OutputUiMode::All;
        self.ui_output_name = None;
        self.power_off_secondary_outputs = false;
        self.secondary_outputs_powered_off = false;
        self.pending_pre_ready_redraw = true;

        for surface in &mut self.lock_surfaces {
            surface.background_path = None;
            surface.background = None;
            surface.scene_base = None;
            surface.scene_base_revision = 0;
            surface.scene_base_has_layers = false;
        }

        Ok(())
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

        if self.outputs_powered_off() || self.secondary_outputs_powered_off {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
            self.secondary_outputs_powered_off = false;
        }

        let Some(session_lock) = self.session_lock.take() else {
            return Ok(());
        };

        if !self.unlock_authorized {
            tracing::error!(
                failure_reason = self.failure_reason.as_deref(),
                "curtain is exiting without a daemon-authorized unlock; leaving the session locked"
            );
            // Leaked on purpose: dropping would send destroy() and can raise invalid_destroy.
            std::mem::forget(session_lock);
            return Ok(());
        }

        if session_lock.is_locked() {
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

    pub(crate) fn note_surface_activity(
        &mut self,
        surface: &wl_surface::WlSurface,
        queue_handle: &QueueHandle<Self>,
    ) {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface)
        else {
            return;
        };

        self.set_focused_surface_index(Some(index), queue_handle);
    }

    fn set_focused_surface_index(
        &mut self,
        index: Option<usize>,
        queue_handle: &QueueHandle<Self>,
    ) {
        if self.focused_surface_index == index {
            return;
        }

        let previous_primary = self.selected_ui_surface_index();
        self.focused_surface_index = index;
        if self.selected_ui_surface_index() != previous_primary {
            self.render_all_surfaces(queue_handle);
        }
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
        self.output_role_for_surface(index).renders_shell()
    }

    pub(crate) fn output_role_for_surface(&self, index: usize) -> OutputRole {
        match self.ui_output_mode {
            OutputUiMode::All => OutputRole::PrimaryPrompt,
            OutputUiMode::Single => {
                if self.selected_ui_surface_index() == Some(index) {
                    OutputRole::PrimaryPrompt
                } else {
                    OutputRole::SecondaryCurtain
                }
            }
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

        if let Some(index) = self.focused_surface_index
            && self
                .lock_surfaces
                .get(index)
                .is_some_and(|surface| surface.size.is_some())
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

pub(crate) fn effective_battery_snapshot(
    config: &AppConfig,
    runtime_snapshot: Option<BatterySnapshot>,
) -> Option<BatterySnapshot> {
    config.battery.mock_snapshot().or(runtime_snapshot)
}

pub(crate) fn effective_weather_location(config: &AppConfig) -> Option<String> {
    config
        .weather
        .enabled
        .then(|| config.weather.normalized_location())
        .flatten()
}

pub(crate) fn effective_weather_snapshot(
    config: &AppConfig,
    runtime_snapshot: Option<WeatherSnapshot>,
) -> Option<WeatherSnapshot> {
    config.weather.enabled.then_some(runtime_snapshot).flatten()
}

#[cfg(test)]
mod tests {
    use super::{
        SurfaceSize, effective_battery_snapshot, effective_weather_location,
        effective_weather_snapshot,
    };
    use veila_common::{AppConfig, BatterySnapshot, WeatherCondition, WeatherSnapshot};

    #[test]
    fn surface_size_tracks_logical_and_scaled_buffer_size() {
        let size = SurfaceSize::new_with_fractional_scale(1920, 1080, 2, None);

        assert_eq!(size.logical_width, 1920);
        assert_eq!(size.logical_height, 1080);
        assert_eq!(size.buffer.width, 3840);
        assert_eq!(size.buffer.height, 2160);
        assert_eq!(size.scale, 2);
        assert_eq!(size.buffer_scale_for_commit(), 2);
    }

    #[test]
    fn surface_size_never_uses_zero_or_negative_scale() {
        let size = SurfaceSize::new_with_fractional_scale(800, 600, 0, None);

        assert_eq!(size.buffer.width, 800);
        assert_eq!(size.buffer.height, 600);
        assert_eq!(size.scale, 1);
    }

    #[test]
    fn fractional_surface_size_commits_with_unit_buffer_scale() {
        let size = SurfaceSize::new_with_fractional_scale(1920, 1080, 2, Some(150));

        assert_eq!(size.buffer.width, 3840);
        assert_eq!(size.buffer.height, 2160);
        assert_eq!(size.buffer_scale_for_commit(), 1);
        assert_eq!(size.fractional_scale, Some(150));
    }

    #[test]
    fn effective_battery_snapshot_prefers_config_mock() {
        let mut config = AppConfig::default();
        config.battery.mock_percent = Some(64);
        config.battery.mock_charging = Some(true);

        assert_eq!(
            effective_battery_snapshot(
                &config,
                Some(BatterySnapshot {
                    percent: 12,
                    charging: false,
                }),
            ),
            Some(BatterySnapshot {
                percent: 64,
                charging: true,
            })
        );
    }

    #[test]
    fn effective_battery_snapshot_uses_runtime_snapshot_without_mock() {
        let config = AppConfig::default();

        assert_eq!(
            effective_battery_snapshot(
                &config,
                Some(BatterySnapshot {
                    percent: 72,
                    charging: false,
                }),
            ),
            Some(BatterySnapshot {
                percent: 72,
                charging: false,
            })
        );
    }

    #[test]
    fn effective_weather_data_is_hidden_when_weather_is_disabled() {
        let mut config = AppConfig::default();
        config.weather.enabled = false;
        config.weather.location = Some(String::from("Riga"));

        assert_eq!(effective_weather_location(&config), None);
        assert_eq!(
            effective_weather_snapshot(
                &config,
                Some(WeatherSnapshot {
                    temperature_celsius: 7,
                    condition: WeatherCondition::Rain,
                    fetched_at_unix: 0,
                }),
            ),
            None
        );
    }

    #[test]
    fn effective_weather_data_uses_runtime_snapshot_when_enabled() {
        let mut config = AppConfig::default();
        config.weather.enabled = true;
        config.weather.location = Some(String::from("Riga"));
        let snapshot = WeatherSnapshot {
            temperature_celsius: 7,
            condition: WeatherCondition::Rain,
            fetched_at_unix: 0,
        };

        assert_eq!(
            effective_weather_location(&config),
            Some(String::from("Riga"))
        );
        assert_eq!(
            effective_weather_snapshot(&config, Some(snapshot.clone())),
            Some(snapshot)
        );
    }
}
