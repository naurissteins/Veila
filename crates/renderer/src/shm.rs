use smithay_client_toolkit::{
    reexports::client::{
        Dispatch, QueueHandle,
        protocol::{wl_buffer, wl_shm, wl_surface::WlSurface},
    },
    shm::{Shm, raw::RawPool},
};

use crate::{FrameSize, RendererError, Result, SoftwareBuffer, SoftwareBufferView};

#[derive(Debug)]
pub struct SurfaceBufferPool {
    pool: RawPool,
}

impl SurfaceBufferPool {
    pub fn new(shm: &Shm, size: crate::FrameSize) -> Result<Self> {
        Ok(Self {
            pool: RawPool::new(required_pool_len(size)?, shm)?,
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
        D: Dispatch<wl_buffer::WlBuffer, ()> + 'static,
    {
        let size = buffer.size();
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        self.pool.resize(byte_len)?;
        self.pool.mmap()[..byte_len].copy_from_slice(buffer.pixels());

        let wl_buffer = self.pool.create_buffer(
            0,
            size.width as i32,
            size.height as i32,
            (size.width * 4) as i32,
            wl_shm::Format::Argb8888,
            (),
            queue_handle,
        );
        surface.set_buffer_scale(buffer_scale.max(1));
        surface.attach(Some(&wl_buffer), 0, 0);
        surface.damage_buffer(0, 0, size.width as i32, size.height as i32);
        surface.commit();
        wl_buffer.destroy();

        Ok(())
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
        D: Dispatch<wl_buffer::WlBuffer, ()> + 'static,
    {
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        self.pool.resize(byte_len)?;
        {
            let mut buffer = SoftwareBufferView::new(size, &mut self.pool.mmap()[..byte_len])?;
            render(&mut buffer)?;
        }

        let wl_buffer = self.pool.create_buffer(
            0,
            size.width as i32,
            size.height as i32,
            (size.width * 4) as i32,
            wl_shm::Format::Argb8888,
            (),
            queue_handle,
        );
        surface.set_buffer_scale(buffer_scale.max(1));
        surface.attach(Some(&wl_buffer), 0, 0);
        surface.damage_buffer(0, 0, size.width as i32, size.height as i32);
        surface.commit();
        wl_buffer.destroy();

        Ok(())
    }
}

pub fn commit_buffer<D>(
    shm: &Shm,
    queue_handle: &QueueHandle<D>,
    surface: &WlSurface,
    buffer: &SoftwareBuffer,
) -> Result<()>
where
    D: Dispatch<wl_buffer::WlBuffer, ()> + 'static,
{
    SurfaceBufferPool::new(shm, buffer.size())?.commit_buffer(queue_handle, surface, buffer, 1)
}

fn required_pool_len(size: crate::FrameSize) -> Result<usize> {
    if size.is_empty() {
        return Err(RendererError::EmptyFrame);
    }

    size.byte_len().ok_or(RendererError::InvalidFrameSize(size))
}
