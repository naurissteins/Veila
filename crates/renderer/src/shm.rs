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
    slot_len: usize,
    next_slot: usize,
}

pub const BUFFER_SLOTS: usize = 2;

impl SurfaceBufferPool {
    pub fn new(shm: &Shm, size: crate::FrameSize) -> Result<Self> {
        let slot_len = required_pool_len(size)?;
        Ok(Self {
            pool: RawPool::new(required_pool_len_for_slots(size, slot_len)?, shm)?,
            slot_len,
            next_slot: 0,
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
        let offset = self.next_buffer_offset(size, byte_len)?;
        self.pool.mmap()[offset..offset + byte_len].copy_from_slice(buffer.pixels());

        let wl_buffer = self.pool.create_buffer(
            offset as i32,
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
        let offset = self.next_buffer_offset(size, byte_len)?;
        {
            let mut buffer =
                SoftwareBufferView::new(size, &mut self.pool.mmap()[offset..offset + byte_len])?;
            render(&mut buffer)?;
        }

        let wl_buffer = self.pool.create_buffer(
            offset as i32,
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
        D: Dispatch<wl_buffer::WlBuffer, ()> + 'static,
    {
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        let offset = self.next_buffer_offset(size, byte_len)?;
        let damaged = {
            let mut buffer =
                SoftwareBufferView::new(size, &mut self.pool.mmap()[offset..offset + byte_len])?;
            render(&mut buffer)?
                .unwrap_or_else(|| damage.clipped_to(size.width as i32, size.height as i32))
        };
        if damaged.is_empty() {
            return Ok(());
        }

        let wl_buffer = self.pool.create_buffer(
            offset as i32,
            size.width as i32,
            size.height as i32,
            (size.width * 4) as i32,
            wl_shm::Format::Argb8888,
            (),
            queue_handle,
        );
        surface.set_buffer_scale(buffer_scale.max(1));
        surface.attach(Some(&wl_buffer), 0, 0);
        surface.damage_buffer(damaged.x, damaged.y, damaged.width, damaged.height);
        surface.commit();
        wl_buffer.destroy();

        Ok(())
    }

    fn next_buffer_offset(&mut self, size: FrameSize, byte_len: usize) -> Result<usize> {
        if self.slot_len != byte_len {
            self.slot_len = byte_len;
            self.next_slot = 0;
            self.pool
                .resize(required_pool_len_for_slots(size, byte_len)?)?;
        }

        let offset = self
            .next_slot
            .checked_mul(byte_len)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        if offset > i32::MAX as usize {
            return Err(RendererError::InvalidFrameSize(size));
        }

        self.next_slot = (self.next_slot + 1) % BUFFER_SLOTS;
        Ok(offset)
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

fn required_pool_len_for_slots(size: FrameSize, byte_len: usize) -> Result<usize> {
    byte_len
        .checked_mul(BUFFER_SLOTS)
        .ok_or(RendererError::InvalidFrameSize(size))
}

#[cfg(test)]
mod tests {
    use crate::FrameSize;

    use super::{BUFFER_SLOTS, required_pool_len, required_pool_len_for_slots};

    #[test]
    fn shm_pool_reserves_multiple_frame_slots() {
        let size = FrameSize::new(64, 32);
        let byte_len = required_pool_len(size).expect("byte len");

        assert_eq!(
            required_pool_len_for_slots(size, byte_len).expect("pool len"),
            byte_len * BUFFER_SLOTS
        );
    }
}
