#![forbid(unsafe_code)]

//! Shared rendering primitives used by Kwylock components.

pub mod background;
pub mod masked;
pub mod panel;
pub mod progress;
pub mod shape;
pub mod shm;
pub mod symbol;
pub mod text;

use thiserror::Error;

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

    /// Returns the ARGB8888 byte length for the frame size.
    pub fn byte_len(self) -> Option<usize> {
        let width = usize::try_from(self.width).ok()?;
        let height = usize::try_from(self.height).ok()?;
        width.checked_mul(height)?.checked_mul(4)
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

    pub const fn to_argb8888_bytes(self) -> [u8; 4] {
        u32::from_be_bytes([self.alpha, self.red, self.green, self.blue]).to_le_bytes()
    }
}

/// Drop-shadow parameters for bitmap primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowStyle {
    pub color: ClearColor,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl ShadowStyle {
    /// Creates a new shadow style.
    pub const fn new(color: ClearColor, offset_x: i32, offset_y: i32) -> Self {
        Self {
            color,
            offset_x,
            offset_y,
        }
    }
}

/// Shared renderer error type.
#[derive(Debug, Error)]
pub enum RendererError {
    #[error("frame size {0:?} is invalid for ARGB8888 rendering")]
    InvalidFrameSize(FrameSize),
    #[error("frame size must not be empty")]
    EmptyFrame,
    #[error(transparent)]
    ShmPool(#[from] smithay_client_toolkit::shm::CreatePoolError),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}

/// Shared result type for rendering operations.
pub type Result<T> = std::result::Result<T, RendererError>;

/// Owned ARGB8888 software buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareBuffer {
    size: FrameSize,
    pixels: Vec<u8>,
}

impl SoftwareBuffer {
    /// Creates a new ARGB8888 buffer of the requested size.
    pub fn new(size: FrameSize) -> Result<Self> {
        let Some(byte_len) = size.byte_len() else {
            return Err(RendererError::InvalidFrameSize(size));
        };

        Ok(Self {
            size,
            pixels: vec![0; byte_len],
        })
    }

    /// Creates a solid-color ARGB8888 buffer.
    pub fn solid(size: FrameSize, color: ClearColor) -> Result<Self> {
        let mut buffer = Self::new(size)?;
        buffer.clear(color);
        Ok(buffer)
    }

    /// Creates a buffer from owned ARGB8888 bytes.
    pub fn from_argb8888_pixels(size: FrameSize, pixels: Vec<u8>) -> Result<Self> {
        let Some(byte_len) = size.byte_len() else {
            return Err(RendererError::InvalidFrameSize(size));
        };

        if pixels.len() != byte_len {
            return Err(RendererError::InvalidFrameSize(size));
        }

        Ok(Self { size, pixels })
    }

    /// Fills the buffer with a single color.
    pub fn clear(&mut self, color: ClearColor) {
        let pixel = color.to_argb8888_bytes();
        self.pixels
            .chunks_exact_mut(4)
            .for_each(|chunk| chunk.copy_from_slice(&pixel));
    }

    /// Returns the frame size for the buffer.
    pub const fn size(&self) -> FrameSize {
        self.size
    }

    /// Returns the ARGB8888 bytes.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns the ARGB8888 bytes for in-place drawing.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
}

#[cfg(test)]
mod tests {
    use super::{ClearColor, FrameSize, SoftwareBuffer};

    #[test]
    fn detects_empty_frame_sizes() {
        assert!(FrameSize::new(0, 1080).is_empty());
        assert!(!FrameSize::new(1920, 1080).is_empty());
    }

    #[test]
    fn computes_argb8888_byte_size() {
        assert_eq!(FrameSize::new(2, 3).byte_len(), Some(24));
    }

    #[test]
    fn fills_solid_buffers() {
        let buffer = SoftwareBuffer::solid(FrameSize::new(2, 1), ClearColor::opaque(10, 20, 30))
            .expect("buffer should be created");

        assert_eq!(buffer.pixels(), &[30, 20, 10, 255, 30, 20, 10, 255]);
    }

    #[test]
    fn creates_buffer_from_argb8888_pixels() {
        let buffer = SoftwareBuffer::from_argb8888_pixels(FrameSize::new(1, 1), vec![4, 3, 2, 1])
            .expect("buffer should be created");

        assert_eq!(buffer.pixels(), &[4, 3, 2, 1]);
    }
}
