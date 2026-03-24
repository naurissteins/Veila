use crate::{
    ClearColor, SoftwareBuffer,
    shape::{Rect, fill_rect},
};

/// Normalized progress value for a progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub current: u32,
    pub max: u32,
}

impl Progress {
    /// Creates a progress value.
    pub const fn new(current: u32, max: u32) -> Self {
        Self { current, max }
    }

    /// Returns the filled width for a track of the given width.
    pub fn filled_width(self, width: i32) -> i32 {
        if width <= 0 || self.current == 0 || self.max == 0 {
            return 0;
        }

        let clamped = self.current.min(self.max) as i64;
        let width = width as i64;
        let filled = (width * clamped) / self.max as i64;

        filled.max(1).min(width) as i32
    }
}

/// Styling for a rectangular progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressBarStyle {
    pub track: ClearColor,
    pub fill: ClearColor,
}

impl ProgressBarStyle {
    /// Creates a progress bar style.
    pub const fn new(track: ClearColor, fill: ClearColor) -> Self {
        Self { track, fill }
    }
}

/// Draws a simple rectangular progress bar.
pub fn draw_progress_bar(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    progress: Progress,
    style: ProgressBarStyle,
) {
    fill_rect(buffer, rect, style.track);

    let filled_width = progress.filled_width(rect.width);
    if filled_width == 0 {
        return;
    }

    fill_rect(
        buffer,
        Rect::new(rect.x, rect.y, filled_width, rect.height),
        style.fill,
    );
}

#[cfg(test)]
mod tests {
    use super::{Progress, ProgressBarStyle, draw_progress_bar};
    use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

    #[test]
    fn computes_progress_fill_width() {
        assert_eq!(Progress::new(1, 2).filled_width(12), 6);
    }

    #[test]
    fn keeps_non_zero_progress_visible() {
        assert_eq!(Progress::new(1, 100).filled_width(3), 1);
    }

    #[test]
    fn renders_progress_bars() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(16, 4)).expect("buffer");
        draw_progress_bar(
            &mut buffer,
            Rect::new(0, 0, 16, 4),
            Progress::new(2, 3),
            ProgressBarStyle::new(
                ClearColor::opaque(16, 20, 28),
                ClearColor::opaque(255, 255, 255),
            ),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
