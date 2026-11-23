mod decoder;
mod geometry;
mod output;
mod power;
mod reader;

use std::time::{Duration, Instant};

use smithay_client_toolkit::reexports::client::{QueueHandle, protocol::wl_output::WlOutput};
use veila_renderer::{FrameSize, RenderScale, SoftwareBuffer, background::BackgroundTreatment};
use veila_ui::StaticBackdrops;

use crate::state::{CurtainApp, RedrawKind, SurfaceSize};

use geometry::VideoGeometry;
use power::PlaybackPower;
pub(crate) use power::VideoHold;
pub(crate) use reader::VideoUpdate;
use reader::{VideoReader, VideoSource};

const INPUT_HOLD: Duration = Duration::from_millis(125);

#[derive(Default)]
pub(crate) struct Video {
    reader: Option<VideoReader>,
    power: PlaybackPower,
    output: Option<output::VideoOutput>,
    target: Option<WlOutput>,
    frame: Option<VideoFrame>,
    input_hold_until: Option<Instant>,
    started: bool,
}

#[derive(Debug)]
pub(crate) struct VideoFrame {
    background: SoftwareBuffer,
    scene_buffer: Option<SoftwareBuffer>,
    geometry: VideoGeometry,
}

impl VideoFrame {
    fn prepare(
        mut frame: SoftwareBuffer,
        geometry: VideoGeometry,
        treatment: BackgroundTreatment,
        backdrops: &StaticBackdrops,
    ) -> Self {
        treatment.apply_color(&mut frame);
        backdrops.render(&mut frame, geometry.scale);
        Self {
            scene_buffer: (!geometry.scaled).then(|| frame.clone()),
            background: frame,
            geometry,
        }
    }

    fn fits(&self, size: SurfaceSize) -> bool {
        self.geometry == VideoGeometry::for_surface(size)
    }

    fn take_scene_buffer(&mut self) -> SoftwareBuffer {
        // keep the painted backdrop for a shell revision change during the input hold
        self.scene_buffer
            .take()
            .unwrap_or_else(|| self.background.clone())
    }
}

impl Video {
    pub(crate) fn targets(&self, output: &WlOutput) -> bool {
        self.target.as_ref() == Some(output)
    }

    pub(crate) fn is_active_for(&self, output: &WlOutput, size: SurfaceSize) -> bool {
        self.frame_for(output, size).is_some()
    }

    pub(crate) fn layer_buffer(
        &self,
        output: &WlOutput,
        size: SurfaceSize,
    ) -> Option<&SoftwareBuffer> {
        let frame = self.frame_for(output, size)?;
        frame.geometry.scaled.then_some(&frame.background)
    }

    fn frame_for(&self, output: &WlOutput, size: SurfaceSize) -> Option<&VideoFrame> {
        self.frame
            .as_ref()
            .filter(|frame| self.targets(output) && frame.fits(size))
    }

    pub(crate) fn take_scene_buffer(
        &mut self,
        output: &WlOutput,
        size: FrameSize,
        scale: RenderScale,
    ) -> Option<SoftwareBuffer> {
        if !self.targets(output) {
            return None;
        }
        self.frame
            .as_mut()
            .filter(|frame| {
                !frame.geometry.scaled
                    && frame.geometry.buffer == size
                    && frame.geometry.scale == scale
            })
            .map(VideoFrame::take_scene_buffer)
    }

    pub(crate) fn prioritize_input(&mut self, now: Instant) {
        if self.reader.is_some() {
            self.input_hold_until = Some(now + INPUT_HOLD);
        }
    }

    fn hold_frame_for_input(&mut self, now: Instant) -> bool {
        let Some(until) = self.input_hold_until else {
            return false;
        };
        if now < until {
            tracing::debug!("video prototype skipped a frame for password input");
            return true;
        }
        self.input_hold_until = None;
        tracing::debug!("video prototype resumed after password input");
        false
    }
}

impl CurtainApp {
    pub(crate) fn start_video_prototype(&mut self, queue: &QueueHandle<Self>) {
        self.video_start_pending = false;
        if self.video.reader.is_some()
            || !self.session_locked
            || !self.ready_notification_sent
            || self.ui_shell.emergency_active()
            || self.slideshow.is_some()
        {
            return;
        }
        let source = self.video_path.clone().map(VideoSource::Clip);
        #[cfg(feature = "video-prototype")]
        let source = source.or_else(|| {
            std::env::var_os("VEILA_VIDEO_PROTOTYPE_FIFO")
                .map(|path| VideoSource::Fifo(path.into()))
        });
        let Some(source) = source else {
            return;
        };
        // shortcut: the fixed-size stream stays on the initial prompt output until video can retarget safely.
        let Some(index) = self.configured_video_surface_index() else {
            self.video_start_pending = self.video_path.is_some();
            return;
        };
        let display_off = self.outputs_powered_off();
        let Some(size) = self.lock_surfaces[index].size else {
            self.video_start_pending = self.video_path.is_some();
            return;
        };
        let geometry = VideoGeometry::for_surface(size);
        if geometry.scaled && !self.can_scale_video(index) {
            tracing::debug!("scaled video requires a Wayland viewport and subsurface");
            return;
        }

        let entry = &mut self.lock_surfaces[index];
        if entry.output_power.is_none() {
            let Ok(manager) = self.output_power_manager.get() else {
                tracing::debug!("video prototype requires output power observation");
                return;
            };
            entry.output_power =
                Some(manager.get_output_power(&entry.output, queue, entry.output.clone()));
        }
        self.video.power.set_hold(VideoHold::Display, true);
        self.video.power.set_hold(
            VideoHold::Display,
            display_off || entry.output_power_off != Some(false),
        );
        match VideoReader::spawn(
            source,
            geometry,
            self.background_treatment,
            self.ui_shell.static_backdrops(),
            self.background_sender.clone(),
            !self.video.power.playing(),
        ) {
            Ok(reader) => {
                self.video.reader = Some(reader);
                self.video.target = Some(entry.output.clone());
                self.video.output = Some(output::VideoOutput::new(
                    self.output_state
                        .info(&entry.output)
                        .and_then(|info| info.name),
                    size,
                ));
                if self.video.power.needs_frame() {
                    self.render_video_surface(queue);
                }
            }
            Err(error) => tracing::warn!(%error, "could not start video prototype"),
        }
    }

    pub(crate) fn accept_video_frame(&mut self, update: VideoUpdate, queue: &QueueHandle<Self>) {
        if !self
            .video
            .reader
            .as_ref()
            .is_some_and(|reader| reader.owns(&update))
            || (update.frame.is_some() && !self.video.power.playing())
        {
            return;
        }
        if let Some(frame) = update.frame
            && let Some(index) = self.video_surface_index()
            && self.ui_visible_on_surface(index)
            && self.lock_surfaces[index]
                .size
                .is_some_and(|size| frame.fits(size))
        {
            // Keep the last scene base and leave SHM slots available for password redraws.
            if self.video.hold_frame_for_input(Instant::now()) {
                return;
            }
            if !self.video.started {
                self.video.started = true;
                tracing::info!("video prototype started after lock readiness");
            } else if self.video.frame.is_none() {
                tracing::debug!("video prototype accepted a frame after wake");
            }
            self.video.frame = Some(frame);
            if let Err(error) = self.present_video_frame(index, queue) {
                tracing::warn!(%error, "could not present video frame, using the static wallpaper");
                self.stop_video_prototype();
                self.render_all_surfaces(queue);
            }
        } else {
            let had_frame = self.stop_video_prototype();
            tracing::info!("video prototype returned to the static wallpaper");
            if had_frame {
                self.render_all_surfaces(queue);
            }
        }
    }

    pub(crate) fn stop_video_prototype(&mut self) -> bool {
        self.video_start_pending = false;
        let had_frame = self.video.frame.is_some();
        if had_frame {
            self.invalidate_video_scene();
        }
        let sleeping = self.video.power.sleeping();
        self.video = Video::default();
        self.video.power.set_hold(VideoHold::Sleep, sleeping);
        had_frame
    }

    pub(crate) fn video_surface_index(&self) -> Option<usize> {
        let target = self.video.target.as_ref()?;
        self.lock_surfaces
            .iter()
            .position(|surface| &surface.output == target)
    }

    pub(crate) fn invalidate_video_scene(&mut self) {
        if let Some(index) = self.video_surface_index() {
            let surface = &mut self.lock_surfaces[index];
            surface.scene_base = None;
            surface.scene_base_revision = 0;
            surface.pending_redraw.request(RedrawKind::Full);
        }
    }
}

#[cfg(test)]
mod tests;
