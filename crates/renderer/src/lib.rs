#![forbid(unsafe_code)]

//! Rendering primitives shared by Kwylock components.

/// Pixel dimensions for a render target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

impl FrameSize {
    /// Creates a new frame size value.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns whether the frame area is empty.
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// RGBA clear color for the lock scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl ClearColor {
    /// Creates an opaque RGB color.
    pub const fn opaque(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: u8::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClearColor, FrameSize};

    #[test]
    fn detects_empty_frame_sizes() {
        assert!(FrameSize::new(0, 1080).is_empty());
        assert!(!FrameSize::new(1920, 1080).is_empty());
    }

    #[test]
    fn creates_opaque_colors() {
        assert_eq!(
            ClearColor::opaque(10, 20, 30),
            ClearColor {
                red: 10,
                green: 20,
                blue: 30,
                alpha: 255,
            }
        );
    }
}
