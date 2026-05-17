use smithay_client_toolkit::{
    reexports::client::{
        Dispatch, QueueHandle,
        protocol::{wl_buffer, wl_shm, wl_surface::WlSurface},
    },
    shm::{Shm, raw::RawPool},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{FrameSize, RendererError, Result, SoftwareBuffer, SoftwareBufferView};

#[derive(Debug)]
pub struct SurfaceBufferPool {
    pool: RawPool,
    slot_len: usize,
    slots: Vec<BufferSlot>,
    next_slot: usize,
}

#[derive(Debug, Clone)]
pub struct ShmBufferRelease {
    released: Arc<AtomicBool>,
}

#[derive(Debug)]
struct BufferSlot {
    released: Arc<AtomicBool>,
    buffer: Option<wl_buffer::WlBuffer>,
}

const MAX_BUFFER_SLOTS: usize = 2;

impl ShmBufferRelease {
    pub fn mark_released(&self) {
        self.released.store(true, Ordering::Release);
    }
}

impl BufferSlot {
    fn new() -> Self {
        Self {
            released: Arc::new(AtomicBool::new(false)),
            buffer: None,
        }
    }
}

impl SurfaceBufferPool {
    pub fn new(shm: &Shm, size: crate::FrameSize) -> Result<Self> {
        let slot_len = required_pool_len(size)?;
        Ok(Self {
            pool: RawPool::new(slot_len, shm)?,
            slot_len,
            slots: Vec::new(),
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
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        let size = buffer.size();
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        let slot_index = self.next_buffer_slot(size, byte_len)?;
        let offset = slot_index
            .checked_mul(byte_len)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        self.pool.mmap()[offset..offset + byte_len].copy_from_slice(buffer.pixels());

        let release = ShmBufferRelease {
            released: self.slots[slot_index].released.clone(),
        };
        let wl_buffer = self.pool.create_buffer(
            offset as i32,
            size.width as i32,
            size.height as i32,
            (size.width * 4) as i32,
            wl_shm::Format::Argb8888,
            release,
            queue_handle,
        );
        surface.set_buffer_scale(buffer_scale.max(1));
        surface.attach(Some(&wl_buffer), 0, 0);
        surface.damage_buffer(0, 0, size.width as i32, size.height as i32);
        surface.commit();
        self.slots[slot_index].buffer = Some(wl_buffer);

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
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        let slot_index = self.next_buffer_slot(size, byte_len)?;
        let offset = slot_index
            .checked_mul(byte_len)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        {
            let mut buffer =
                SoftwareBufferView::new(size, &mut self.pool.mmap()[offset..offset + byte_len])?;
            render(&mut buffer)?;
        }

        let release = ShmBufferRelease {
            released: self.slots[slot_index].released.clone(),
        };
        let wl_buffer = self.pool.create_buffer(
            offset as i32,
            size.width as i32,
            size.height as i32,
            (size.width * 4) as i32,
            wl_shm::Format::Argb8888,
            release,
            queue_handle,
        );
        surface.set_buffer_scale(buffer_scale.max(1));
        surface.attach(Some(&wl_buffer), 0, 0);
        surface.damage_buffer(0, 0, size.width as i32, size.height as i32);
        surface.commit();
        self.slots[slot_index].buffer = Some(wl_buffer);

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
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        let slot_index = self.next_buffer_slot(size, byte_len)?;
        let offset = slot_index
            .checked_mul(byte_len)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        let damaged = {
            let mut buffer =
                SoftwareBufferView::new(size, &mut self.pool.mmap()[offset..offset + byte_len])?;
            render(&mut buffer)?
                .unwrap_or_else(|| damage.clipped_to(size.width as i32, size.height as i32))
        };
        if damaged.is_empty() {
            self.slots[slot_index]
                .released
                .store(true, Ordering::Release);
            return Ok(());
        }

        let release = ShmBufferRelease {
            released: self.slots[slot_index].released.clone(),
        };
        let wl_buffer = self.pool.create_buffer(
            offset as i32,
            size.width as i32,
            size.height as i32,
            (size.width * 4) as i32,
            wl_shm::Format::Argb8888,
            release,
            queue_handle,
        );
        surface.set_buffer_scale(buffer_scale.max(1));
        surface.attach(Some(&wl_buffer), 0, 0);
        surface.damage_buffer(damaged.x, damaged.y, damaged.width, damaged.height);
        surface.commit();
        self.slots[slot_index].buffer = Some(wl_buffer);

        Ok(())
    }

    pub fn reserved_bytes(&self) -> usize {
        reserved_bytes_for_slots(self.slot_len, self.slots.len())
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    fn next_buffer_slot(&mut self, size: FrameSize, byte_len: usize) -> Result<usize> {
        if self.slot_len != byte_len {
            self.slot_len = byte_len;
            self.slots.clear();
            self.next_slot = 0;
            self.pool.resize(byte_len)?;
        }

        self.compact_released_slots(size, byte_len)?;

        if let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.released.load(Ordering::Acquire))
        {
            slot.buffer = None;
            slot.released.store(false, Ordering::Release);
            self.next_slot = (index + 1) % self.slots.len().max(1);
            return Ok(index);
        }

        if self.slots.len() >= MAX_BUFFER_SLOTS {
            let index = self.next_slot % self.slots.len();
            if let Some(buffer) = self.slots[index].buffer.take() {
                buffer.destroy();
            }
            self.slots[index].released.store(false, Ordering::Release);
            self.next_slot = (index + 1) % self.slots.len();
            return Ok(index);
        }

        let index = self.slots.len();
        let new_len = byte_len
            .checked_mul(index + 1)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        if new_len > i32::MAX as usize {
            return Err(RendererError::InvalidFrameSize(size));
        }
        self.pool.resize(new_len)?;
        self.slots.push(BufferSlot::new());
        self.next_slot = (index + 1) % self.slots.len().max(1);
        Ok(index)
    }

    fn compact_released_slots(&mut self, size: FrameSize, byte_len: usize) -> Result<()> {
        while self.slots.len() > 1
            && self
                .slots
                .last()
                .is_some_and(|slot| slot.released.load(Ordering::Acquire))
        {
            self.slots.pop();
        }
        if !self.slots.is_empty() {
            self.next_slot %= self.slots.len();
        }

        let new_len = byte_len
            .checked_mul(self.slots.len().max(1))
            .ok_or(RendererError::InvalidFrameSize(size))?;
        self.pool.resize(new_len)?;
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
    D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
{
    SurfaceBufferPool::new(shm, buffer.size())?.commit_buffer(queue_handle, surface, buffer, 1)
}

fn required_pool_len(size: crate::FrameSize) -> Result<usize> {
    if size.is_empty() {
        return Err(RendererError::EmptyFrame);
    }

    size.byte_len().ok_or(RendererError::InvalidFrameSize(size))
}

fn reserved_bytes_for_slots(slot_len: usize, slots: usize) -> usize {
    slot_len.saturating_mul(slots)
}

#[cfg(test)]
mod tests {
    use crate::FrameSize;

    use super::{MAX_BUFFER_SLOTS, required_pool_len, reserved_bytes_for_slots};

    #[test]
    fn required_pool_len_matches_frame_byte_len() {
        let size = FrameSize::new(64, 32);

        assert_eq!(required_pool_len(size).expect("byte len"), 64 * 32 * 4);
    }

    #[test]
    fn reports_reserved_bytes_from_live_slots() {
        assert_eq!(reserved_bytes_for_slots(64 * 32 * 4, 2), 64 * 32 * 4 * 2);
    }

    #[test]
    fn pool_growth_is_capped_to_two_frame_slots() {
        assert_eq!(MAX_BUFFER_SLOTS, 2);
    }
}
