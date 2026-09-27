use std::{
    os::fd::AsFd,
    time::{Duration, Instant},
};

use nix::fcntl::{FallocateFlags, fallocate};

use super::{PoolGeneration, SurfaceBufferPool, slots::SlotState};
use crate::{RendererError, Result};

// Longer than the 1 Hz countdown redraws so a steady ticker never trims and refaults a slot.
const TRIM_QUIET_WINDOW: Duration = Duration::from_secs(2);

fn is_trimmable(state: SlotState, index: usize, last_committed: Option<usize>) -> bool {
    state == SlotState::Released && last_committed != Some(index)
}

fn quiet_remaining(last_commit_at: Option<Instant>, now: Instant) -> Duration {
    last_commit_at
        .map(|at| TRIM_QUIET_WINDOW.saturating_sub(now.saturating_duration_since(at)))
        .unwrap_or(Duration::ZERO)
}

// Punching a hole frees the shmem pages for every mapping; later reads fault in zero pages.
fn punch_slot(fd: impl AsFd, slot_len: usize, index: usize) -> Result<()> {
    let offset = index
        .checked_mul(slot_len)
        .and_then(|offset| i64::try_from(offset).ok())
        .ok_or(RendererError::SlotTrim(nix::errno::Errno::EOVERFLOW))?;
    let len = i64::try_from(slot_len)
        .map_err(|_| RendererError::SlotTrim(nix::errno::Errno::EOVERFLOW))?;
    fallocate(
        fd,
        FallocateFlags::FALLOC_FL_PUNCH_HOLE | FallocateFlags::FALLOC_FL_KEEP_SIZE,
        offset,
        len,
    )
    .map_err(RendererError::SlotTrim)
}

impl SurfaceBufferPool {
    /// Time until released slots may be trimmed, or `None` when nothing is trimmable.
    pub fn trim_due_in(&self, now: Instant) -> Option<Duration> {
        if self.trim_disabled || !self.current.has_trimmable_slots() {
            return None;
        }
        Some(quiet_remaining(self.last_commit_at, now))
    }

    /// Returns released, non-current slot pages to the kernel once the pool has been quiet.
    pub fn trim_released(&mut self, now: Instant) -> Result<usize> {
        self.collect_released();
        if self.trim_due_in(now) != Some(Duration::ZERO) {
            return Ok(0);
        }

        let generation = &mut self.current;
        let mut trimmed = 0;
        for index in 0..generation.slots.len() {
            let state = generation.slots[index].state();
            if !is_trimmable(state, index, generation.last_committed_slot) {
                continue;
            }
            if let Err(error) = punch_slot(&generation.pool, generation.slot_len, index) {
                self.trim_disabled = true;
                return Err(error);
            }
            generation.slots[index].mark_trimmed();
            trimmed += generation.slot_len;
        }
        Ok(trimmed)
    }
}

impl PoolGeneration {
    fn has_trimmable_slots(&self) -> bool {
        self.slots
            .iter()
            .enumerate()
            .any(|(index, slot)| is_trimmable(slot.state(), index, self.last_committed_slot))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom, Write},
        time::{Duration, Instant},
    };

    use super::{SlotState, TRIM_QUIET_WINDOW, is_trimmable, punch_slot, quiet_remaining};

    #[test]
    fn trims_only_released_slots_that_are_not_last_committed() {
        assert!(is_trimmable(SlotState::Released, 0, Some(1)));
        assert!(is_trimmable(SlotState::Released, 0, None));
        assert!(!is_trimmable(SlotState::Released, 1, Some(1)));
        assert!(!is_trimmable(SlotState::Busy, 0, Some(1)));
        assert!(!is_trimmable(SlotState::Trimmed, 0, Some(1)));
    }

    #[test]
    fn waits_for_the_quiet_window_after_a_commit() {
        let committed_at = Instant::now();

        assert_eq!(
            quiet_remaining(Some(committed_at), committed_at),
            TRIM_QUIET_WINDOW
        );
        assert_eq!(
            quiet_remaining(
                Some(committed_at),
                committed_at + Duration::from_millis(400)
            ),
            TRIM_QUIET_WINDOW - Duration::from_millis(400)
        );
        assert_eq!(
            quiet_remaining(Some(committed_at), committed_at + TRIM_QUIET_WINDOW),
            Duration::ZERO
        );
        assert_eq!(quiet_remaining(None, committed_at), Duration::ZERO);
    }

    #[test]
    fn punching_a_slot_zeroes_only_that_slot() {
        let mut file = tempfile_in_shm();
        file.write_all(&[7_u8; 3 * 4096]).expect("fill file");

        punch_slot(&file, 4096, 1).expect("punch slot");

        let mut pixels = Vec::new();
        file.seek(SeekFrom::Start(0)).expect("rewind");
        file.read_to_end(&mut pixels).expect("read file");
        assert_eq!(pixels.len(), 3 * 4096);
        assert!(pixels[..4096].iter().all(|byte| *byte == 7));
        assert!(pixels[4096..2 * 4096].iter().all(|byte| *byte == 0));
        assert!(pixels[2 * 4096..].iter().all(|byte| *byte == 7));
    }

    fn tempfile_in_shm() -> File {
        let name = format!(
            "veila-trim-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        );
        let fd =
            nix::sys::memfd::memfd_create(name.as_str(), nix::sys::memfd::MFdFlags::MFD_CLOEXEC)
                .expect("create memfd");
        File::from(fd)
    }
}
