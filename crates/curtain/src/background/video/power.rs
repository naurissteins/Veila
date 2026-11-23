use smithay_client_toolkit::reexports::client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_callback, wl_surface},
};

use super::{super::CurtainApp, Video};

#[derive(Clone, Copy)]
pub(crate) enum VideoHold {
    Display,
    Sleep,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Playing,
    Held,
    NeedsFrame,
    AwaitingCallback,
}

#[derive(Default)]
pub(super) struct PlaybackPower {
    display_off: bool,
    sleeping: bool,
    phase: Phase,
    callback: Option<wl_callback::WlCallback>,
}

impl PlaybackPower {
    pub(super) fn playing(&self) -> bool {
        self.phase == Phase::Playing
    }

    pub(super) fn needs_frame(&self) -> bool {
        self.phase == Phase::NeedsFrame
    }

    pub(super) fn sleeping(&self) -> bool {
        self.sleeping
    }

    pub(super) fn set_hold(&mut self, reason: VideoHold, held: bool) -> bool {
        let before = self.phase == Phase::Held;
        match reason {
            VideoHold::Display => self.display_off = held,
            VideoHold::Sleep => self.sleeping = held,
        }
        if self.display_off || self.sleeping {
            self.phase = Phase::Held;
            self.callback = None;
        } else if before {
            self.phase = Phase::NeedsFrame;
        }
        before != (self.phase == Phase::Held)
    }

    fn request_callback(&mut self) -> bool {
        if !self.needs_frame() {
            return false;
        }
        self.phase = Phase::AwaitingCallback;
        true
    }

    fn resume_after_callback(&mut self) -> bool {
        if self.phase != Phase::AwaitingCallback {
            return false;
        }
        self.phase = Phase::Playing;
        true
    }
}

pub(crate) struct VideoWake;

impl Video {
    pub(crate) fn request_wake_callback(
        &mut self,
        surface: &wl_surface::WlSurface,
        queue: &QueueHandle<CurtainApp>,
    ) {
        if self.reader.is_some() && self.power.request_callback() {
            self.power.callback = Some(surface.frame(queue, VideoWake));
            tracing::debug!("video prototype queued a static wake frame");
        }
    }
}

impl Dispatch<wl_callback::WlCallback, VideoWake> for CurtainApp {
    fn event(
        app: &mut Self,
        callback: &wl_callback::WlCallback,
        _event: wl_callback::Event,
        _data: &VideoWake,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let video = &mut app.video;
        if video.power.callback.as_ref() == Some(callback)
            && let Some(reader) = &video.reader
            && video.power.resume_after_callback()
        {
            video.power.callback = None;
            reader.set_paused(false);
            tracing::info!("video prototype resumed after static wake frame");
        }
    }
}

impl CurtainApp {
    pub(crate) fn update_video_hold(&mut self, reason: VideoHold, held: bool) -> bool {
        if matches!(reason, VideoHold::Sleep)
            && self.video.power.sleeping() != held
            && let Some(output) = &mut self.video.output
        {
            output.sleep_changed(held, std::time::Instant::now());
        }
        let changed = self.video.power.set_hold(reason, held);
        let Some(reader) = &self.video.reader else {
            return false;
        };
        if !changed {
            return false;
        }
        if held {
            reader.set_paused(true);
            let had_frame = self.video.frame.take().is_some();
            if had_frame {
                self.invalidate_video_scene();
            }
            tracing::info!("video prototype paused for power transition");
            had_frame
        } else {
            tracing::debug!("video prototype waiting for static wake frame");
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_waits_for_both_wake_conditions_and_a_fresh_frame_callback() {
        let mut power = PlaybackPower::default();
        assert!(power.playing());
        assert!(power.set_hold(VideoHold::Display, true));
        assert!(!power.set_hold(VideoHold::Sleep, true));
        assert!(!power.set_hold(VideoHold::Sleep, false));
        assert!(!power.request_callback());
        assert!(power.set_hold(VideoHold::Display, false));
        assert!(power.request_callback());
        assert!(power.set_hold(VideoHold::Sleep, true));
        assert!(!power.resume_after_callback());
        assert!(power.set_hold(VideoHold::Sleep, false));
        assert!(power.request_callback());
        assert!(power.resume_after_callback());
        assert!(power.playing());
    }
}
