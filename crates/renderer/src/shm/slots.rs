use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use smithay_client_toolkit::reexports::client::protocol::wl_buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SlotState {
    Busy,
    Released,
    Trimmed,
}

#[derive(Debug)]
pub(super) struct BufferSlot {
    released: Arc<AtomicBool>,
    trimmed: bool,
    buffer: Option<wl_buffer::WlBuffer>,
}

impl BufferSlot {
    pub(super) fn new() -> Self {
        Self {
            released: Arc::new(AtomicBool::new(false)),
            trimmed: false,
            buffer: None,
        }
    }

    pub(super) fn state(&self) -> SlotState {
        if !self.released.load(Ordering::Acquire) {
            SlotState::Busy
        } else if self.trimmed {
            SlotState::Trimmed
        } else {
            SlotState::Released
        }
    }

    pub(super) fn release_flag(&self) -> Arc<AtomicBool> {
        self.released.clone()
    }

    pub(super) fn prepare_for_reuse(&mut self) {
        self.destroy_buffer();
        self.trimmed = false;
        self.released.store(false, Ordering::Release);
    }

    pub(super) fn abandon_render(&self) {
        self.released.store(true, Ordering::Release);
    }

    pub(super) fn set_buffer(&mut self, buffer: wl_buffer::WlBuffer) {
        self.buffer = Some(buffer);
    }

    pub(super) fn mark_trimmed(&mut self) {
        self.destroy_buffer();
        self.trimmed = true;
    }

    fn destroy_buffer(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            buffer.destroy();
        }
    }
}

impl Drop for BufferSlot {
    fn drop(&mut self) {
        self.destroy_buffer();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SlotChoice {
    Reuse(usize),
    Grow,
    Skip,
}

pub(super) fn choose_slot(
    states: &[SlotState],
    last_committed: Option<usize>,
    max_slots: usize,
) -> SlotChoice {
    let released = |index: &usize| states[*index] == SlotState::Released;
    let preferred = last_committed
        .filter(|index| *index < states.len())
        .filter(released)
        .or_else(|| (0..states.len()).find(released))
        .or_else(|| states.iter().position(|state| *state == SlotState::Trimmed));
    if let Some(index) = preferred {
        return SlotChoice::Reuse(index);
    }

    if states.len() >= max_slots {
        SlotChoice::Skip
    } else {
        SlotChoice::Grow
    }
}

pub(super) fn copy_slot_pixels(
    pixels: &mut [u8],
    slot_len: usize,
    source: usize,
    target: usize,
) -> bool {
    let Some(source_start) = source.checked_mul(slot_len) else {
        return false;
    };
    let Some(source_end) = source_start.checked_add(slot_len) else {
        return false;
    };
    let Some(target_start) = target.checked_mul(slot_len) else {
        return false;
    };
    let Some(target_end) = target_start.checked_add(slot_len) else {
        return false;
    };
    if source_end > pixels.len() || target_end > pixels.len() {
        return false;
    }

    pixels.copy_within(source_start..source_end, target_start);
    true
}

#[cfg(test)]
mod tests {
    use super::{
        SlotChoice::{Grow, Reuse, Skip},
        SlotState::{Busy, Released, Trimmed},
        choose_slot, copy_slot_pixels,
    };

    const MAX_BUFFER_SLOTS: usize = 2;

    #[test]
    fn grows_while_under_the_slot_cap() {
        assert_eq!(choose_slot(&[], None, MAX_BUFFER_SLOTS), Grow);
        assert_eq!(choose_slot(&[Busy], Some(0), MAX_BUFFER_SLOTS), Grow);
    }

    #[test]
    fn reuses_the_first_released_slot() {
        assert_eq!(
            choose_slot(&[Busy, Released], Some(0), MAX_BUFFER_SLOTS),
            Reuse(1)
        );
        assert_eq!(
            choose_slot(&[Released, Released], None, MAX_BUFFER_SLOTS),
            Reuse(0)
        );
    }

    #[test]
    fn prefers_the_released_last_committed_slot() {
        assert_eq!(
            choose_slot(&[Released, Released], Some(1), MAX_BUFFER_SLOTS),
            Reuse(1)
        );
        assert_eq!(
            choose_slot(&[Trimmed, Released], Some(1), MAX_BUFFER_SLOTS),
            Reuse(1)
        );
    }

    #[test]
    fn reuses_a_trimmed_slot_before_growing_or_skipping() {
        assert_eq!(
            choose_slot(&[Trimmed, Busy], Some(1), MAX_BUFFER_SLOTS),
            Reuse(0)
        );
        assert_eq!(choose_slot(&[Trimmed], None, MAX_BUFFER_SLOTS), Reuse(0));
    }

    #[test]
    fn skips_the_frame_rather_than_overwriting_a_busy_slot() {
        assert_eq!(choose_slot(&[Busy, Busy], Some(1), MAX_BUFFER_SLOTS), Skip);
    }

    #[test]
    fn restores_a_reused_slot_from_the_last_committed_slot() {
        let mut pixels = [1, 2, 3, 4, 9, 9, 9, 9];

        assert!(copy_slot_pixels(&mut pixels, 4, 0, 1));
        assert_eq!(pixels, [1, 2, 3, 4, 1, 2, 3, 4]);
    }
}
