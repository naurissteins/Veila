use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use veila_common::config::{BackgroundConfig, BackgroundSlideshowMode, BackgroundSlideshowOrder, wallpaper_paths_equal};

pub(crate) struct BackgroundSlideshow {
    paths: Vec<PathBuf>,
    mode: BackgroundSlideshowMode,
    order: BackgroundSlideshowOrder,
    sequence: Vec<usize>,
    position: usize,
    change_interval: Duration,
    transition_duration: Duration,
    next_change_at: Option<Instant>,
}

impl BackgroundSlideshow {
    pub(crate) fn load(background: &BackgroundConfig, initial_path: Option<&Path>) -> Option<Self> {
        if !background.slideshow_enabled() {
            return None;
        }

        let slideshow = background.slideshow.as_ref()?;
        let paths = match background.resolved_slideshow_paths() {
            Ok(paths) => paths,
            Err(error) => {
                tracing::warn!("failed to resolve slideshow wallpaper sources: {error:#}");
                return None;
            }
        };

        if paths.is_empty() {
            return None;
        }

        let order = slideshow.order;
        let mode = slideshow.mode;
        let initial_path = initial_path
            .map(ToOwned::to_owned)
            .or_else(|| background.resolved_slideshow_initial_path().ok().flatten());
        let initial_index = initial_path
            .as_deref()
            .and_then(|initial_path| {
                paths
                    .iter()
                    .position(|path| wallpaper_paths_equal(path, initial_path))
            })
            .unwrap_or_else(|| {
                if initial_path.is_some() {
                    tracing::warn!(
                        requested = initial_path
                            .as_deref()
                            .map(|path| path.display().to_string()),
                        "daemon-selected slideshow wallpaper was not found in resolved paths; falling back to first entry"
                    );
                }
                0
            });
        let sequence = slideshow_sequence(paths.len(), order, initial_index);

        let next_change_at = (paths.len() > 1 && slideshow.rotates_while_locked())
            .then(|| Instant::now() + slideshow.change_interval());

        Some(Self {
            paths,
            mode,
            order,
            sequence,
            position: 0,
            change_interval: slideshow.change_interval(),
            transition_duration: slideshow.transition_duration(),
            next_change_at,
        })
    }

    pub(crate) fn transition_duration(&self) -> Duration {
        self.transition_duration
    }

    pub(crate) fn current_path(&self) -> &Path {
        &self.paths[self.sequence[self.position]]
    }

    pub(crate) fn len(&self) -> usize {
        self.paths.len()
    }

    pub(crate) fn next_due_in(&self, now: Instant) -> Option<Duration> {
        self.next_change_at
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    pub(crate) fn advance(&mut self, now: Instant) -> Option<PathBuf> {
        let next_change_at = self.next_change_at?;
        if now < next_change_at || self.paths.len() < 2 {
            return None;
        }

        self.position += 1;
        if self.position >= self.sequence.len() {
            self.reset_sequence();
        }

        self.next_change_at = Some(now + self.change_interval);
        Some(self.current_path().to_path_buf())
    }

    pub(crate) fn next_preload_path(&self) -> Option<PathBuf> {
        if self.paths.len() < 2 || !matches!(self.mode, BackgroundSlideshowMode::Timed) {
            return None;
        }

        let mut next_position = self.position + 1;
        if next_position >= self.sequence.len() {
            next_position = 0;
        }

        Some(self.paths[self.sequence[next_position]].clone())
    }

    fn reset_sequence(&mut self) {
        let current_index = self.sequence[self.sequence.len() - 1];
        self.sequence = slideshow_sequence(self.paths.len(), self.order, current_index);
        self.position = 0;
    }
}

fn slideshow_sequence(
    len: usize,
    order: BackgroundSlideshowOrder,
    initial_index: usize,
) -> Vec<usize> {
    let mut sequence = Vec::with_capacity(len);
    sequence.push(initial_index);

    match order {
        BackgroundSlideshowOrder::Sequence => {
            for offset in 1..len {
                sequence.push((initial_index + offset) % len);
            }
        }
        BackgroundSlideshowOrder::Random => {
            let mut tail: Vec<_> = (0..len).filter(|index| *index != initial_index).collect();
            shuffle(&mut tail);
            sequence.extend(tail);
        }
    }

    sequence
}

fn shuffle(sequence: &mut [usize]) {
    let mut state = shuffle_seed();
    for index in (1..sequence.len()).rev() {
        let candidate = next_u64(&mut state) as usize % (index + 1);
        sequence.swap(index, candidate);
    }
}

fn shuffle_seed() -> u64 {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    seed ^ 0x9E37_79B9_7F4A_7C15
}

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        time::{Duration, Instant},
    };

    use super::{BackgroundSlideshow, shuffle, slideshow_sequence};

    #[test]
    fn random_shuffle_keeps_all_items() {
        let mut sequence = vec![0, 1, 2, 3];
        shuffle(&mut sequence);

        assert_eq!(sequence.len(), 4);
        assert!(sequence.contains(&0));
        assert!(sequence.contains(&1));
        assert!(sequence.contains(&2));
        assert!(sequence.contains(&3));
    }

    #[test]
    fn random_sequence_keeps_initial_item_first() {
        let sequence =
            slideshow_sequence(4, veila_common::config::BackgroundSlideshowOrder::Random, 2);

        assert_eq!(sequence[0], 2);
        assert_eq!(sequence.len(), 4);
        assert!(sequence.contains(&0));
        assert!(sequence.contains(&1));
        assert!(sequence.contains(&3));
    }

    #[test]
    fn next_preload_wraps_to_start() {
        let mut slideshow = BackgroundSlideshow {
            paths: vec!["/tmp/one.jpg".into(), "/tmp/two.jpg".into()],
            mode: veila_common::config::BackgroundSlideshowMode::Timed,
            order: veila_common::config::BackgroundSlideshowOrder::Sequence,
            sequence: vec![0, 1],
            position: 1,
            change_interval: Duration::from_secs(5),
            transition_duration: Duration::from_millis(800),
            next_change_at: Some(Instant::now()),
        };

        assert_eq!(
            slideshow.next_preload_path().as_deref(),
            Some(Path::new("/tmp/one.jpg"))
        );

        slideshow.reset_sequence();
        assert_eq!(slideshow.position, 0);
    }

    #[test]
    fn lock_only_mode_disables_next_preload_and_timed_rotation() {
        let mut slideshow = BackgroundSlideshow {
            paths: vec!["/tmp/one.jpg".into(), "/tmp/two.jpg".into()],
            mode: veila_common::config::BackgroundSlideshowMode::LockOnly,
            order: veila_common::config::BackgroundSlideshowOrder::Random,
            sequence: vec![0, 1],
            position: 0,
            change_interval: Duration::from_secs(5),
            transition_duration: Duration::from_millis(800),
            next_change_at: None,
        };

        assert!(slideshow.next_preload_path().is_none());
        assert!(slideshow.advance(Instant::now()).is_none());
    }
}
