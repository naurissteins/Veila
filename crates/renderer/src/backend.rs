#[cfg(feature = "gpu")]
use std::{ffi::c_void, ptr::NonNull};

use smithay_client_toolkit::{
    reexports::client::{
        Dispatch, QueueHandle,
        protocol::{wl_buffer, wl_surface::WlSurface},
    },
    shm::Shm,
};

use crate::{
    FrameSize, Result, SoftwareBuffer, SoftwareBufferView,
    shm::{ShmBufferRelease, SurfaceBufferPool},
};

#[cfg(feature = "gpu")]
mod gpu;

#[cfg(feature = "gpu")]
use gpu::GpuFrameBackend;

const GPU_NOT_COMPILED_REASON: &str = "gpu feature is not compiled in";
#[cfg(feature = "gpu")]
pub(crate) const GPU_NO_DISPLAY_REASON: &str = "wayland display handle is unavailable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameBackendKind {
    Software,
    Gpu,
}

impl FrameBackendKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Software => "software",
            Self::Gpu => "gpu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameBackendPreference {
    Software,
    Gpu,
    Auto,
}

impl FrameBackendPreference {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Software => "software",
            Self::Gpu => "gpu",
            Self::Auto => "auto",
        }
    }

    pub fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "software" | "shm" | "cpu" => Some(Self::Software),
            "gpu" => Some(Self::Gpu),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct FrameBackendSelection {
    pub backend: FrameBackend,
    pub requested: FrameBackendPreference,
    pub fallback_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameBackendContext {
    #[cfg(feature = "gpu")]
    wayland_display: Option<NonNull<c_void>>,
}

#[derive(Debug)]
pub enum FrameBackend {
    Software(SoftwareFrameBackend),
    #[cfg(feature = "gpu")]
    Gpu(Box<GpuFrameBackend>),
}

#[derive(Debug)]
pub struct SoftwareFrameBackend {
    pool: SurfaceBufferPool,
}

impl FrameBackendContext {
    pub const fn software_only() -> Self {
        Self {
            #[cfg(feature = "gpu")]
            wayland_display: None,
        }
    }

    #[cfg(feature = "gpu")]
    pub const fn wayland_display(wayland_display: NonNull<c_void>) -> Self {
        Self {
            wayland_display: Some(wayland_display),
        }
    }
}

impl FrameBackend {
    pub fn new_software(shm: &Shm, size: FrameSize) -> Result<Self> {
        Ok(Self::Software(SoftwareFrameBackend::new(shm, size)?))
    }

    pub fn new_preferred(
        shm: &Shm,
        context: FrameBackendContext,
        size: FrameSize,
        preference: FrameBackendPreference,
    ) -> Result<FrameBackendSelection> {
        match preference {
            FrameBackendPreference::Software => Ok(FrameBackendSelection {
                backend: Self::new_software(shm, size)?,
                requested: preference,
                fallback_reason: None,
            }),
            FrameBackendPreference::Gpu | FrameBackendPreference::Auto => {
                let gpu_attempt = Self::try_gpu_backend(shm, context, size);
                match gpu_attempt {
                    Ok(backend) => Ok(FrameBackendSelection {
                        backend,
                        requested: preference,
                        fallback_reason: None,
                    }),
                    Err(reason) => Ok(FrameBackendSelection {
                        backend: Self::new_software(shm, size)?,
                        requested: preference,
                        fallback_reason: fallback_reason(preference, reason),
                    }),
                }
            }
        }
    }

    pub const fn kind(&self) -> FrameBackendKind {
        match self {
            Self::Software(_) => FrameBackendKind::Software,
            #[cfg(feature = "gpu")]
            Self::Gpu(_) => FrameBackendKind::Gpu,
        }
    }

    pub fn commit_buffer<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        buffer: &SoftwareBuffer,
        buffer_scale: i32,
    ) -> Result<()>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        match self {
            Self::Software(backend) => {
                backend.commit_buffer(queue_handle, surface, buffer, buffer_scale)
            }
            #[cfg(feature = "gpu")]
            Self::Gpu(backend) => {
                backend.commit_buffer(queue_handle, surface, buffer, buffer_scale)
            }
        }
    }

    pub fn render_buffer<D>(
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
        match self {
            Self::Software(backend) => {
                backend.render_buffer(queue_handle, surface, size, buffer_scale, render)
            }
            #[cfg(feature = "gpu")]
            Self::Gpu(backend) => {
                backend.render_buffer(queue_handle, surface, size, buffer_scale, render)
            }
        }
    }

    pub fn render_buffer_region<D>(
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
        match self {
            Self::Software(backend) => backend.render_buffer_region(
                queue_handle,
                surface,
                size,
                buffer_scale,
                damage,
                render,
            ),
            #[cfg(feature = "gpu")]
            Self::Gpu(backend) => backend.render_buffer_region(
                queue_handle,
                surface,
                size,
                buffer_scale,
                damage,
                render,
            ),
        }
    }

    pub fn reserved_bytes(&self) -> usize {
        match self {
            Self::Software(backend) => backend.reserved_bytes(),
            #[cfg(feature = "gpu")]
            Self::Gpu(backend) => backend.reserved_bytes(),
        }
    }

    pub fn slot_count(&self) -> usize {
        match self {
            Self::Software(backend) => backend.slot_count(),
            #[cfg(feature = "gpu")]
            Self::Gpu(backend) => backend.slot_count(),
        }
    }

    #[cfg(feature = "gpu")]
    fn try_gpu_backend(
        shm: &Shm,
        context: FrameBackendContext,
        size: FrameSize,
    ) -> std::result::Result<Self, &'static str> {
        GpuFrameBackend::new(shm, context, size).map(|backend| Self::Gpu(Box::new(backend)))
    }

    #[cfg(not(feature = "gpu"))]
    fn try_gpu_backend(
        _shm: &Shm,
        _context: FrameBackendContext,
        _size: FrameSize,
    ) -> std::result::Result<Self, &'static str> {
        Err(GPU_NOT_COMPILED_REASON)
    }
}

impl SoftwareFrameBackend {
    pub fn new(shm: &Shm, size: FrameSize) -> Result<Self> {
        Ok(Self {
            pool: SurfaceBufferPool::new(shm, size)?,
        })
    }

    pub fn commit_buffer<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        buffer: &SoftwareBuffer,
        buffer_scale: i32,
    ) -> Result<()>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        self.pool
            .commit_buffer(queue_handle, surface, buffer, buffer_scale)
    }

    pub fn render_buffer<D>(
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
        self.pool
            .render_buffer(queue_handle, surface, size, buffer_scale, render)
    }

    pub fn render_buffer_region<D>(
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
        self.pool
            .render_buffer_region(queue_handle, surface, size, buffer_scale, damage, render)
    }

    pub fn reserved_bytes(&self) -> usize {
        self.pool.reserved_bytes()
    }

    pub fn slot_count(&self) -> usize {
        self.pool.slot_count()
    }
}

fn fallback_reason(
    preference: FrameBackendPreference,
    reason: &'static str,
) -> Option<&'static str> {
    match (preference, reason) {
        (FrameBackendPreference::Auto, GPU_NOT_COMPILED_REASON) => None,
        _ => Some(reason),
    }
}

#[cfg(test)]
mod tests;
