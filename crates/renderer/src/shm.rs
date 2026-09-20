use smithay_client_toolkit::{
    error::GlobalError,
    globals::ProvidesBoundGlobal,
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

mod slots;

use slots::{BufferSlot, SlotChoice, choose_slot, copy_slot_pixels};

#[derive(Debug)]
pub struct SurfaceBufferPool {
    shm: ShmHandle,
    current: PoolGeneration,
    retired: Vec<PoolGeneration>,
}

#[derive(Debug, Clone)]
pub struct ShmBufferRelease {
    released: Arc<AtomicBool>,
    surface: WlSurface,
}

#[derive(Debug)]
struct ShmHandle(wl_shm::WlShm);

#[derive(Debug)]
struct PoolGeneration {
    pool: RawPool,
    slot_len: usize,
    slots: Vec<BufferSlot>,
    last_committed_slot: Option<usize>,
}

const MAX_BUFFER_SLOTS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameResult {
    Committed,
    Skipped,
}

impl ShmBufferRelease {
    pub fn mark_released(&self) {
        self.released.store(true, Ordering::Release);
    }

    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }
}

impl ProvidesBoundGlobal<wl_shm::WlShm, 1> for ShmHandle {
    fn bound_global(&self) -> std::result::Result<wl_shm::WlShm, GlobalError> {
        Ok(self.0.clone())
    }
}

impl PoolGeneration {
    fn new(shm: &ShmHandle, size: FrameSize) -> Result<Self> {
        let slot_len = required_pool_len(size)?;
        Ok(Self {
            pool: RawPool::new(slot_len, shm)?,
            slot_len,
            slots: Vec::new(),
            last_committed_slot: None,
        })
    }

    fn has_busy_slots(&self) -> bool {
        self.slots.iter().any(|slot| !slot.is_released())
    }
}

impl SurfaceBufferPool {
    pub fn new(shm: &Shm, size: crate::FrameSize) -> Result<Self> {
        let shm = ShmHandle(shm.wl_shm().clone());
        Ok(Self {
            current: PoolGeneration::new(&shm, size)?,
            shm,
            retired: Vec::new(),
        })
    }

    pub fn commit_buffer<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        buffer: &SoftwareBuffer,
        buffer_scale: i32,
    ) -> Result<FrameResult>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        let size = buffer.size();
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        let Some(slot_index) = self.next_buffer_slot(size, byte_len)? else {
            return Ok(FrameResult::Skipped);
        };
        let offset = slot_index
            .checked_mul(byte_len)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        self.current.pool.mmap()[offset..offset + byte_len].copy_from_slice(buffer.pixels());

        let release = ShmBufferRelease {
            released: self.current.slots[slot_index].release_flag(),
            surface: surface.clone(),
        };
        let wl_buffer = self.current.pool.create_buffer(
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
        self.current.slots[slot_index].set_buffer(wl_buffer);
        self.current.last_committed_slot = Some(slot_index);

        Ok(FrameResult::Committed)
    }

    pub fn render_buffer<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        size: FrameSize,
        buffer_scale: i32,
        render: impl FnOnce(&mut SoftwareBufferView<'_>) -> Result<()>,
    ) -> Result<FrameResult>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        let Some(slot_index) = self.next_buffer_slot(size, byte_len)? else {
            return Ok(FrameResult::Skipped);
        };
        let offset = slot_index
            .checked_mul(byte_len)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        {
            let mut buffer = SoftwareBufferView::new(
                size,
                &mut self.current.pool.mmap()[offset..offset + byte_len],
            )?;
            if let Err(error) = render(&mut buffer) {
                self.current.slots[slot_index].abandon_render();
                return Err(error);
            }
        }

        let release = ShmBufferRelease {
            released: self.current.slots[slot_index].release_flag(),
            surface: surface.clone(),
        };
        let wl_buffer = self.current.pool.create_buffer(
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
        self.current.slots[slot_index].set_buffer(wl_buffer);
        self.current.last_committed_slot = Some(slot_index);

        Ok(FrameResult::Committed)
    }

    pub fn render_buffer_region<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        size: FrameSize,
        buffer_scale: i32,
        damage: crate::shape::Rect,
        render: impl FnOnce(&mut SoftwareBufferView<'_>) -> Result<Option<crate::shape::Rect>>,
    ) -> Result<FrameResult>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        if size.is_empty() {
            return Err(RendererError::EmptyFrame);
        }

        let byte_len = required_pool_len(size)?;
        let Some(slot_index) = self.next_buffer_slot(size, byte_len)? else {
            return Ok(FrameResult::Skipped);
        };
        let offset = slot_index
            .checked_mul(byte_len)
            .ok_or(RendererError::InvalidFrameSize(size))?;
        let Some(previous) = self.current.last_committed_slot else {
            self.current.slots[slot_index].abandon_render();
            return Err(RendererError::MissingCommittedFrame);
        };
        if previous != slot_index
            && !copy_slot_pixels(self.current.pool.mmap(), byte_len, previous, slot_index)
        {
            self.current.slots[slot_index].abandon_render();
            return Err(RendererError::InvalidFrameSize(size));
        }
        let damaged = {
            let mut buffer = SoftwareBufferView::new(
                size,
                &mut self.current.pool.mmap()[offset..offset + byte_len],
            )?;
            match render(&mut buffer) {
                Ok(damaged) => damaged
                    .unwrap_or_else(|| damage.clipped_to(size.width as i32, size.height as i32)),
                Err(error) => {
                    self.current.slots[slot_index].abandon_render();
                    return Err(error);
                }
            }
        };
        if damaged.is_empty() {
            self.current.slots[slot_index].abandon_render();
            return Ok(FrameResult::Committed);
        }

        let release = ShmBufferRelease {
            released: self.current.slots[slot_index].release_flag(),
            surface: surface.clone(),
        };
        let wl_buffer = self.current.pool.create_buffer(
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
        self.current.slots[slot_index].set_buffer(wl_buffer);
        self.current.last_committed_slot = Some(slot_index);

        Ok(FrameResult::Committed)
    }

    pub fn reserved_bytes(&self) -> usize {
        std::iter::once(&self.current)
            .chain(self.retired.iter())
            .map(|generation| reserved_bytes_for_slots(generation.slot_len, generation.slots.len()))
            .sum()
    }

    pub fn slot_count(&self) -> usize {
        self.current.slots.len()
            + self
                .retired
                .iter()
                .map(|generation| generation.slots.len())
                .sum::<usize>()
    }

    pub fn collect_released(&mut self) {
        self.retired.retain(PoolGeneration::has_busy_slots);
    }

    fn next_buffer_slot(&mut self, size: FrameSize, byte_len: usize) -> Result<Option<usize>> {
        self.collect_released();
        if self.current.slot_len != byte_len {
            let next = PoolGeneration::new(&self.shm, size)?;
            let previous = std::mem::replace(&mut self.current, next);
            if previous.has_busy_slots() {
                self.retired.push(previous);
            }
        }

        let choice = choose_slot(
            self.current.slots.iter().map(BufferSlot::is_released),
            self.current.slots.len(),
            MAX_BUFFER_SLOTS,
        );

        match choice {
            SlotChoice::Reuse(index) => {
                self.current.slots[index].prepare_for_reuse();
                Ok(Some(index))
            }
            SlotChoice::Skip => Ok(None),
            SlotChoice::Grow => {
                let index = self.current.slots.len();
                let new_len = byte_len
                    .checked_mul(index + 1)
                    .ok_or(RendererError::InvalidFrameSize(size))?;
                if new_len > i32::MAX as usize {
                    return Err(RendererError::InvalidFrameSize(size));
                }
                self.current.pool.resize(new_len)?;
                self.current.slots.push(BufferSlot::new());
                Ok(Some(index))
            }
        }
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
    SurfaceBufferPool::new(shm, buffer.size())?
        .commit_buffer(queue_handle, surface, buffer, 1)
        .map(|_| ())
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
