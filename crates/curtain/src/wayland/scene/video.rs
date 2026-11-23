use anyhow::{Result, anyhow};
use smithay_client_toolkit::{
    compositor::Region,
    reexports::{
        client::{
            Proxy, QueueHandle,
            protocol::{
                wl_subcompositor::WlSubcompositor, wl_subsurface::WlSubsurface,
                wl_surface::WlSurface,
            },
        },
        protocols::wp::viewporter::client::wp_viewport::WpViewport,
    },
};
use veila_renderer::{FrameSize, SoftwareBuffer, shm};

use crate::state::{CurtainApp, SurfaceSize};

pub(crate) struct VideoLayer {
    surface: WlSurface,
    subsurface: WlSubsurface,
    viewport: WpViewport,
    pool: shm::SurfaceBufferPool,
    mapped: bool,
}

impl VideoLayer {
    fn new(
        app: &CurtainApp,
        parent: &WlSurface,
        size: SurfaceSize,
        buffer: FrameSize,
        queue: &QueueHandle<CurtainApp>,
    ) -> Result<Self> {
        let viewporter = app.viewporter.get()?;
        let pool = shm::SurfaceBufferPool::new(&app.shm, buffer)?;
        let input = Region::new(&app.compositor_state)?;
        let subcompositor: WlSubcompositor = app.registry_state.bind_one(queue, 1..=1, ())?;
        let surface = app.compositor_state.create_surface(queue);
        let subsurface = subcompositor.get_subsurface(&surface, parent, queue, ());
        subcompositor.destroy();
        subsurface.place_below(parent);
        surface.set_input_region(Some(input.wl_region()));
        let viewport = viewporter.get_viewport(&surface, queue, ());
        viewport.set_destination(size.logical_width as i32, size.logical_height as i32);
        Ok(Self {
            surface,
            subsurface,
            viewport,
            pool,
            mapped: false,
        })
    }

    fn commit(
        &mut self,
        frame: &SoftwareBuffer,
        queue: &QueueHandle<CurtainApp>,
    ) -> Result<shm::FrameResult> {
        Ok(self.pool.commit_buffer(queue, &self.surface, frame, 1)?)
    }

    pub(super) fn hide_on_parent_commit(&self) {
        self.subsurface.set_sync();
        self.surface.attach(None, 0, 0);
        self.surface.commit();
    }

    pub(super) fn parent_committed(&mut self) {
        if !self.mapped {
            // first video frame and transparent UI take effect in the same parent commit
            self.subsurface.set_desync();
            self.mapped = true;
        }
    }
}

impl Drop for VideoLayer {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

impl CurtainApp {
    pub(crate) fn can_scale_video(&self, index: usize) -> bool {
        self.lock_surfaces[index].viewport.is_some()
            && self
                .registry_state
                .globals_by_interface(WlSubcompositor::interface().name)
                .next()
                .is_some()
    }

    pub(crate) fn present_video_frame(
        &mut self,
        index: usize,
        queue: &QueueHandle<Self>,
    ) -> Result<()> {
        let entry = &self.lock_surfaces[index];
        let size = entry
            .size
            .ok_or_else(|| anyhow!("video output is unconfigured"))?;
        let surface = entry.surface.clone();
        let Some(frame) = self.video.layer_buffer(&entry.output, size) else {
            self.invalidate_video_scene();
            return self.render_surface(&surface, size, queue);
        };

        if let Some(layer) = self.lock_surfaces[index].video_layer.as_mut() {
            if layer.mapped {
                layer.commit(frame, queue)?;
            }
            return Ok(());
        }

        let mut layer = VideoLayer::new(self, surface.wl_surface(), size, frame.size(), queue)?;
        if layer.commit(frame, queue)? == shm::FrameResult::Committed {
            tracing::debug!(
                width = frame.size().width,
                height = frame.size().height,
                ui_width = size.buffer.width,
                ui_height = size.buffer.height,
                "staged a scaled video background"
            );
            self.lock_surfaces[index].video_layer = Some(layer);
            self.invalidate_video_scene();
            self.render_surface(&surface, size, queue)?;
        }
        Ok(())
    }

    pub(super) fn uses_video_layer(&self, index: usize, size: SurfaceSize) -> bool {
        let entry = &self.lock_surfaces[index];
        entry.video_layer.is_some() && self.video.layer_buffer(&entry.output, size).is_some()
    }
}
