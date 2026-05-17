use std::{ffi::c_void, ptr::NonNull};

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::reexports::client::{Proxy, protocol::wl_surface::WlSurface};

use super::pipeline::TextureCompositor;
use crate::{FrameSize, Result};

#[derive(Debug)]
pub(super) struct GpuSurfaceState {
    pub(super) surface_id: usize,
    pub(super) size: FrameSize,
    pub(super) surface: wgpu::Surface<'static>,
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) compositor: TextureCompositor,
}

impl GpuSurfaceState {
    pub(super) fn new(
        wayland_display: NonNull<c_void>,
        wl_surface: &WlSurface,
        size: FrameSize,
    ) -> Result<Self> {
        let surface_id = wl_surface.id().as_ptr() as usize;
        let wayland_surface = NonNull::new(wl_surface.id().as_ptr().cast()).ok_or_else(|| {
            crate::RendererError::FrameBackendUnavailable("wayland surface is null".to_string())
        })?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let raw_display_handle =
            RawDisplayHandle::Wayland(WaylandDisplayHandle::new(wayland_display));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(wayland_surface));
        let surface = create_wgpu_surface(&instance, raw_display_handle, raw_window_handle)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| {
            crate::RendererError::FrameBackendUnavailable(
                "no compatible gpu adapter found".to_string(),
            )
        })?;
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default(), None))
            .map_err(|error| crate::RendererError::FrameBackendUnavailable(error.to_string()))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| {
                crate::RendererError::FrameBackendUnavailable(
                    "gpu surface has no supported formats".to_string(),
                )
            })?;
        let present_mode = if capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![format],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let compositor = TextureCompositor::new(&device, format);

        Ok(Self {
            surface_id,
            size,
            surface,
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            config,
            compositor,
        })
    }

    pub(super) fn current_texture(&mut self) -> Result<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            Ok(texture) => Ok(texture),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface.get_current_texture().map_err(surface_error)
            }
            Err(error) => Err(surface_error(error)),
        }
    }
}

#[allow(unsafe_code)]
fn create_wgpu_surface(
    instance: &wgpu::Instance,
    raw_display_handle: RawDisplayHandle,
    raw_window_handle: RawWindowHandle,
) -> Result<wgpu::Surface<'static>> {
    // SAFETY: CurtainApp owns the live Wayland display and matching session-lock surface.
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle,
            raw_window_handle,
        })
    }
    .map_err(|error| crate::RendererError::FrameBackendUnavailable(error.to_string()))?;

    Ok(surface)
}

fn surface_error(error: wgpu::SurfaceError) -> crate::RendererError {
    crate::RendererError::FrameBackendUnavailable(error.to_string())
}
