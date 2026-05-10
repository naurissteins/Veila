use std::{collections::HashSet, sync::Arc};

use crate::state::CurtainApp;

struct SurfaceMemorySummary {
    output: String,
    width: u32,
    height: u32,
    ui_visible: bool,
    software_buffer_count: u8,
    software_buffers_kib: u64,
    shm_pool_estimated_kib: u64,
    estimated_persistent_kib: u64,
    has_background: bool,
    has_scene_base: bool,
}

struct CurtainMemorySummary {
    rss_kib: Option<u64>,
    software_buffers_kib: u64,
    shm_pool_estimated_kib: u64,
    estimated_persistent_kib: u64,
    ui_visible_surfaces: usize,
    surfaces: Vec<SurfaceMemorySummary>,
}

impl CurtainApp {
    pub(crate) fn log_memory_snapshot(&self, phase: &'static str) {
        let summary = self.memory_summary();
        tracing::info!(
            phase,
            surfaces = summary.surfaces.len(),
            ui_visible_surfaces = summary.ui_visible_surfaces,
            rss_kib = summary.rss_kib,
            software_buffers_kib = summary.software_buffers_kib,
            shm_pool_estimated_kib = summary.shm_pool_estimated_kib,
            estimated_persistent_kib = summary.estimated_persistent_kib,
            "curtain memory summary"
        );

        for surface in summary.surfaces {
            tracing::debug!(
                phase,
                output = surface.output,
                width = surface.width,
                height = surface.height,
                ui_visible = surface.ui_visible,
                software_buffer_count = surface.software_buffer_count,
                software_buffers_kib = surface.software_buffers_kib,
                shm_pool_estimated_kib = surface.shm_pool_estimated_kib,
                estimated_persistent_kib = surface.estimated_persistent_kib,
                has_background = surface.has_background,
                has_scene_base = surface.has_scene_base,
                "curtain surface memory summary"
            );
        }
    }

    pub(crate) fn note_memory_after_render(&mut self, first_frame: bool) {
        if !self.ready_notified || self.post_ready_memory_logged || first_frame {
            return;
        }

        self.post_ready_nonfirst_renders = self.post_ready_nonfirst_renders.saturating_add(1);
        let threshold = self.lock_surfaces.len().saturating_mul(2).max(6) as u32;
        if self.post_ready_nonfirst_renders < threshold {
            return;
        }

        self.log_memory_snapshot("post-ready-redraw");
        self.post_ready_memory_logged = true;
    }

    fn memory_summary(&self) -> CurtainMemorySummary {
        let mut software_buffers_kib = 0_u64;
        let mut shm_pool_estimated_kib = 0_u64;
        let mut ui_visible_surfaces = 0_usize;
        let mut surfaces = Vec::with_capacity(self.lock_surfaces.len());
        let mut counted_scene_bases = HashSet::with_capacity(self.lock_surfaces.len());

        for (index, surface) in self.lock_surfaces.iter().enumerate() {
            let Some(size) = surface.size else {
                continue;
            };

            let frame_size = size.buffer;
            let frame_kib = frame_size
                .byte_len()
                .map(|byte_len| (byte_len / 1024) as u64)
                .unwrap_or(0);
            let ui_visible = self.ui_visible_on_surface(index);
            ui_visible_surfaces += usize::from(ui_visible);

            let background_kib = software_buffer_kib(surface.background.as_ref());
            let scene_base_kib = software_buffer_kib(surface.scene_base.as_deref());
            let software_total_kib = background_kib + scene_base_kib;
            let shm_kib = u64::from(surface.shm_pool.is_some()) * frame_kib;

            software_buffers_kib = software_buffers_kib.saturating_add(background_kib);
            if let Some(scene_base) = surface.scene_base.as_ref()
                && counted_scene_bases.insert(Arc::as_ptr(scene_base))
            {
                software_buffers_kib = software_buffers_kib.saturating_add(scene_base_kib);
            }
            shm_pool_estimated_kib = shm_pool_estimated_kib.saturating_add(shm_kib);

            let output = self
                .output_state
                .info(&surface.output)
                .and_then(|info| info.name.clone())
                .unwrap_or_else(|| format!("surface-{index}"));
            let software_buffer_count =
                u8::from(surface.background.is_some()) + u8::from(surface.scene_base.is_some());

            surfaces.push(SurfaceMemorySummary {
                output,
                width: frame_size.width,
                height: frame_size.height,
                ui_visible,
                software_buffer_count,
                software_buffers_kib: software_total_kib,
                shm_pool_estimated_kib: shm_kib,
                estimated_persistent_kib: software_total_kib.saturating_add(shm_kib),
                has_background: surface.background.is_some(),
                has_scene_base: surface.scene_base.is_some(),
            });
        }

        CurtainMemorySummary {
            rss_kib: current_rss_kib(),
            software_buffers_kib,
            shm_pool_estimated_kib,
            estimated_persistent_kib: software_buffers_kib.saturating_add(shm_pool_estimated_kib),
            ui_visible_surfaces,
            surfaces,
        }
    }
}

fn software_buffer_kib(buffer: Option<&veila_renderer::SoftwareBuffer>) -> u64 {
    buffer
        .and_then(|buffer| buffer.size().byte_len())
        .map(|byte_len| (byte_len / 1024) as u64)
        .unwrap_or(0)
}

#[cfg(target_os = "linux")]
fn current_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(parse_vm_rss_kib)
}

#[cfg(not(target_os = "linux"))]
fn current_rss_kib() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn parse_vm_rss_kib(line: &str) -> Option<u64> {
    let value = line.strip_prefix("VmRSS:")?.trim();
    let number = value.split_whitespace().next()?;
    number.parse().ok()
}
