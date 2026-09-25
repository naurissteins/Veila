use smithay_client_toolkit::{
    error::GlobalError,
    globals::ProvidesBoundGlobal,
    reexports::client::{
        Dispatch, QueueHandle,
        protocol::{wl_buffer, wl_shm, wl_surface::WlSurface},
    },
    shm::{Shm, raw::RawPool},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::{FrameSize, RendererError, Result, SoftwareBuffer, SoftwareBufferView, shape::Rect};

mod slots;
mod trim;

use slots::{BufferSlot, SlotChoice, SlotState, choose_slot, copy_slot_pixels};

#[derive(Debug)]
pub struct SurfaceBufferPool {
    shm: ShmHandle,
    current: PoolGeneration,
    retired: Vec<PoolGeneration>,
    last_commit_at: Option<Instant>,
    trim_disabled: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShmPoolMemory {
    pub slots: usize,
    pub current_bytes: usize,
    pub trimmed_bytes: usize,
    pub retired_bytes: usize,
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
        self.slots
            .iter()
            .any(|slot| slot.state() == SlotState::Busy)
    }

    fn slot_states(&self) -> Vec<SlotState> {
        self.slots.iter().map(BufferSlot::state).collect()
    }

    fn slot_offset(&self, index: usize, size: FrameSize) -> Result<usize> {
        index
            .checked_mul(self.slot_len)
            .ok_or(RendererError::InvalidFrameSize(size))
    }
}

impl SurfaceBufferPool {
    pub fn new(shm: &Shm, size: crate::FrameSize) -> Result<Self> {
        let shm = ShmHandle(shm.wl_shm().clone());
        Ok(Self {
            current: PoolGeneration::new(&shm, size)?,
            shm,
            retired: Vec::new(),
            last_commit_at: None,
            trim_disabled: false,
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
        let byte_len = required_pool_len(size)?;
        let Some(slot_index) = self.next_buffer_slot(size, byte_len)? else {
            return Ok(FrameResult::Skipped);
        };
        let offset = self.current.slot_offset(slot_index, size)?;
        self.current.pool.mmap()[offset..offset + byte_len].copy_from_slice(buffer.pixels());

        let damage = Rect::new(0, 0, size.width as i32, size.height as i32);
        self.attach_slot(
            queue_handle,
            surface,
            size,
            buffer_scale,
            slot_index,
            damage,
        );
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
        let byte_len = required_pool_len(size)?;
        let Some(slot_index) = self.next_buffer_slot(size, byte_len)? else {
            return Ok(FrameResult::Skipped);
        };
        let offset = self.current.slot_offset(slot_index, size)?;
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

        let damage = Rect::new(0, 0, size.width as i32, size.height as i32);
        self.attach_slot(
            queue_handle,
            surface,
            size,
            buffer_scale,
            slot_index,
            damage,
        );
        Ok(FrameResult::Committed)
    }

    pub fn render_buffer_region<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        size: FrameSize,
        buffer_scale: i32,
        damage: Rect,
        render: impl FnOnce(&mut SoftwareBufferView<'_>) -> Result<Option<Rect>>,
    ) -> Result<FrameResult>
    where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        let byte_len = required_pool_len(size)?;
        let Some(slot_index) = self.next_buffer_slot(size, byte_len)? else {
            return Ok(FrameResult::Skipped);
        };
        let offset = self.current.slot_offset(slot_index, size)?;
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

        self.attach_slot(
            queue_handle,
            surface,
            size,
            buffer_scale,
            slot_index,
            damaged,
        );
        Ok(FrameResult::Committed)
    }

    pub fn memory(&self) -> ShmPoolMemory {
        let mut memory = ShmPoolMemory {
            slots: self.current.slots.len(),
            ..ShmPoolMemory::default()
        };
        for slot in &self.current.slots {
            if slot.state() == SlotState::Trimmed {
                memory.trimmed_bytes += self.current.slot_len;
            } else {
                memory.current_bytes += self.current.slot_len;
            }
        }
        for generation in &self.retired {
            memory.slots += generation.slots.len();
            memory.retired_bytes += generation.slot_len.saturating_mul(generation.slots.len());
        }
        memory
    }

    pub fn collect_released(&mut self) {
        self.retired.retain(PoolGeneration::has_busy_slots);
    }

    fn attach_slot<D>(
        &mut self,
        queue_handle: &QueueHandle<D>,
        surface: &WlSurface,
        size: FrameSize,
        buffer_scale: i32,
        slot_index: usize,
        damage: Rect,
    ) where
        D: Dispatch<wl_buffer::WlBuffer, ShmBufferRelease> + 'static,
    {
        let slot_len = self.current.slot_len;
        let release = ShmBufferRelease {
            released: self.current.slots[slot_index].release_flag(),
            surface: surface.clone(),
        };
        let wl_buffer = self.current.pool.create_buffer(
            (slot_index * slot_len) as i32,
            size.width as i32,
            size.height as i32,
            (size.width * 4) as i32,
            wl_shm::Format::Argb8888,
            release,
            queue_handle,
        );
        surface.set_buffer_scale(buffer_scale.max(1));
        surface.attach(Some(&wl_buffer), 0, 0);
        surface.damage_buffer(damage.x, damage.y, damage.width, damage.height);
        surface.commit();
        self.current.slots[slot_index].set_buffer(wl_buffer);
        self.current.last_committed_slot = Some(slot_index);
        self.last_commit_at = Some(Instant::now());
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
            &self.current.slot_states(),
            self.current.last_committed_slot,
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

fn required_pool_len(size: crate::FrameSize) -> Result<usize> {
    if size.is_empty() {
        return Err(RendererError::EmptyFrame);
    }

    size.byte_len().ok_or(RendererError::InvalidFrameSize(size))
}

#[cfg(test)]
mod tests {
    use crate::FrameSize;

    use super::{MAX_BUFFER_SLOTS, required_pool_len};

    #[test]
    fn required_pool_len_matches_frame_byte_len() {
        let size = FrameSize::new(64, 32);

        assert_eq!(required_pool_len(size).expect("byte len"), 64 * 32 * 4);
    }

    #[test]
    fn pool_growth_is_capped_to_two_frame_slots() {
        assert_eq!(MAX_BUFFER_SLOTS, 2);
    }
}
