use std::{ffi::c_void, ptr::NonNull};

use smithay_client_toolkit::{
    reexports::client::{
        Dispatch, Proxy, QueueHandle,
        protocol::{wl_buffer, wl_surface::WlSurface},
    },
    shm::Shm,
};

use super::{FrameBackendContext, GPU_NO_DISPLAY_REASON, SoftwareFrameBackend};
use crate::{FrameSize, Result, SoftwareBuffer, SoftwareBufferView, shm::ShmBufferRelease};

mod color;
mod fallback;
mod pipeline;
mod surface;
#[cfg(test)]
mod tests;

use color::{clear_color_from_bgra_pixel, solid_clear_pixel_from_buffer};
use fallback::catch_gpu_panic;
use pipeline::validate_upload_buffer;
use surface::GpuSurfaceState;

#[derive(Debug)]
pub struct GpuFrameBackend {
    wayland_display: NonNull<c_void>,
    size: FrameSize,
    software: SoftwareFrameBackend,
    staging: Option<SoftwareBuffer>,
    surface: Option<GpuSurfaceState>,
    disabled_reason: Option<String>,
    software_bootstrap_done: bool,
}

impl GpuFrameBackend {
    pub(super) fn new(
        shm: &Shm,
        context: FrameBackendContext,
        size: FrameSize,
    ) -> std::result::Result<Self, &'static str> {
        if size.is_empty() {
            return Err("gpu backend requires a non-empty frame");
        }
        let Some(wayland_display) = context.wayland_display else {
            return Err(GPU_NO_DISPLAY_REASON);
        };

        Ok(Self {
            wayland_display,
            size,
            software: SoftwareFrameBackend::new(shm, size)
                .map_err(|_| "software fallback backend is unavailable")?,
            staging: None,
            surface: None,
            disabled_reason: None,
            software_bootstrap_done: false,
        })
    }

    pub(super) fn commit_buffer<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        buffer: &SoftwareBuffer,
        buffer_scale: i32,
    ) -> Result<()>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        if self.take_software_bootstrap() {
            return self
                .software
                .commit_buffer(queue_handle, surface, buffer, buffer_scale);
        }

        if self.gpu_disabled() {
            return self
                .software
                .commit_buffer(queue_handle, surface, buffer, buffer_scale);
        }

        match self.present_buffer(surface, buffer, buffer_scale) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.disable_after_error(&error);
                tracing::warn!(error = %error, "gpu frame commit failed; using software fallback");
                self.software
                    .commit_buffer(queue_handle, surface, buffer, buffer_scale)
            }
        }
    }

    pub(super) fn render_buffer<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        size: FrameSize,
        buffer_scale: i32,
        render: impl FnOnce(&mut SoftwareBufferView<'_>) -> Result<()>,
    ) -> Result<()>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        let mut staging = self.take_staging(size)?;
        {
            let mut view = SoftwareBufferView::new(size, staging.pixels_mut())?;
            render(&mut view)?;
        }

        if self.take_software_bootstrap() {
            self.software
                .commit_buffer(queue_handle, surface, &staging, buffer_scale)?;
            self.staging = Some(staging);
            return Ok(());
        }

        if self.gpu_disabled() {
            self.software
                .commit_buffer(queue_handle, surface, &staging, buffer_scale)?;
            self.staging = Some(staging);
            return Ok(());
        }

        if let Err(error) = self.present_uploaded_buffer(surface, &staging, buffer_scale) {
            self.disable_after_error(&error);
            tracing::warn!(error = %error, "gpu frame render failed; using software fallback");
            self.software
                .commit_buffer(queue_handle, surface, &staging, buffer_scale)?;
        }

        self.staging = Some(staging);
        Ok(())
    }

    pub(super) fn render_buffer_region<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        size: FrameSize,
        buffer_scale: i32,
        damage: crate::shape::Rect,
        render: impl FnOnce(&mut SoftwareBufferView<'_>) -> Result<Option<crate::shape::Rect>>,
    ) -> Result<()>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        if self
            .staging
            .as_ref()
            .is_none_or(|staging| staging.size() != size)
        {
            self.take_software_bootstrap();
            return self.software.render_buffer_region(
                queue_handle,
                surface,
                size,
                buffer_scale,
                damage,
                render,
            );
        }

        let mut staging = self.staging.take().expect("staging buffer should exist");
        let damaged = {
            let mut view = SoftwareBufferView::new(size, staging.pixels_mut())?;
            render(&mut view)?
                .unwrap_or_else(|| damage.clipped_to(size.width as i32, size.height as i32))
        };
        if damaged.is_empty() {
            self.staging = Some(staging);
            return Ok(());
        }

        if self.take_software_bootstrap() {
            self.software
                .commit_buffer(queue_handle, surface, &staging, buffer_scale)?;
            self.staging = Some(staging);
            return Ok(());
        }

        if self.gpu_disabled() {
            self.software
                .commit_buffer(queue_handle, surface, &staging, buffer_scale)?;
            self.staging = Some(staging);
            return Ok(());
        }

        if let Err(error) = self.present_uploaded_buffer(surface, &staging, buffer_scale) {
            self.disable_after_error(&error);
            tracing::warn!(error = %error, "gpu dirty render failed; using software fallback");
            self.software
                .commit_buffer(queue_handle, surface, &staging, buffer_scale)?;
        }

        self.staging = Some(staging);
        Ok(())
    }

    pub(super) fn reserved_bytes(&self) -> usize {
        self.software.reserved_bytes().saturating_add(
            self.staging
                .as_ref()
                .map_or(0, |buffer| buffer.pixels().len()),
        )
    }

    pub(super) fn slot_count(&self) -> usize {
        self.software.slot_count()
    }

    fn present_clear(
        &mut self,
        surface: &WlSurface,
        size: FrameSize,
        buffer_scale: i32,
        pixel: [u8; 4],
    ) -> Result<()> {
        self.ensure_gpu_enabled()?;

        catch_gpu_panic(|| {
            self.ensure_surface(surface, size)?;

            let state = self.surface.as_mut().ok_or_else(|| {
                crate::RendererError::FrameBackendUnavailable(
                    "gpu surface is unavailable".to_string(),
                )
            })?;
            let color = clear_color_from_bgra_pixel(pixel, state.config.format);
            surface.set_buffer_scale(buffer_scale.max(1));

            let surface_texture = state.current_texture()?;
            let texture_view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("veila-gpu-clear-encoder"),
                    });

            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("veila-gpu-clear-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }

            state.queue.submit(Some(encoder.finish()));
            surface_texture.present();
            Ok(())
        })
    }

    fn present_buffer(
        &mut self,
        surface: &WlSurface,
        buffer: &SoftwareBuffer,
        buffer_scale: i32,
    ) -> Result<()> {
        validate_upload_buffer(buffer)?;

        if let Some(pixel) = solid_clear_pixel_from_buffer(buffer) {
            return self.present_clear(surface, buffer.size(), buffer_scale, pixel);
        }

        self.present_uploaded_buffer(surface, buffer, buffer_scale)
    }

    fn present_uploaded_buffer(
        &mut self,
        surface: &WlSurface,
        buffer: &SoftwareBuffer,
        buffer_scale: i32,
    ) -> Result<()> {
        self.ensure_gpu_enabled()?;
        validate_upload_buffer(buffer)?;

        catch_gpu_panic(|| {
            self.ensure_surface(surface, buffer.size())?;

            let state = self.surface.as_mut().ok_or_else(|| {
                crate::RendererError::FrameBackendUnavailable(
                    "gpu surface is unavailable".to_string(),
                )
            })?;
            surface.set_buffer_scale(buffer_scale.max(1));
            let frame_texture = state
                .compositor
                .upload_frame(&state.device, &state.queue, buffer);
            let surface_texture = state.current_texture()?;
            let target = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("veila-gpu-frame-encoder"),
                    });
            state
                .compositor
                .encode(&mut encoder, &target, &frame_texture);
            state.queue.submit(Some(encoder.finish()));
            surface_texture.present();
            Ok(())
        })
    }

    fn ensure_surface(&mut self, wl_surface: &WlSurface, size: FrameSize) -> Result<()> {
        let surface_id = wl_surface.id().as_ptr() as usize;
        if self
            .surface
            .as_ref()
            .is_some_and(|surface| surface.surface_id == surface_id && surface.size == size)
        {
            return Ok(());
        }

        self.surface = Some(GpuSurfaceState::new(
            self.wayland_display,
            wl_surface,
            size,
        )?);
        self.size = size;
        Ok(())
    }

    fn take_staging(&mut self, size: FrameSize) -> Result<SoftwareBuffer> {
        match self.staging.take() {
            Some(buffer) if buffer.size() == size => Ok(buffer),
            _ => SoftwareBuffer::new(size),
        }
    }

    fn ensure_gpu_enabled(&self) -> Result<()> {
        match &self.disabled_reason {
            Some(reason) => Err(crate::RendererError::FrameBackendUnavailable(format!(
                "gpu backend disabled after previous failure: {reason}"
            ))),
            None => Ok(()),
        }
    }

    fn gpu_disabled(&self) -> bool {
        self.disabled_reason.is_some()
    }

    fn take_software_bootstrap(&mut self) -> bool {
        let should_bootstrap = !self.software_bootstrap_done;
        self.software_bootstrap_done = true;
        should_bootstrap
    }

    fn disable_after_error(&mut self, error: &crate::RendererError) {
        if self.disabled_reason.is_none() {
            self.disabled_reason = Some(error.to_string());
            self.surface = None;
        }
    }
}
