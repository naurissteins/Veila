#![forbid(unsafe_code)]

//! Shared rendering primitives used by Veila components.

mod blur;

pub mod background;
pub mod draw;
pub mod shm;

// Re-export draw submodules at the crate root for ergonomic access.
pub use draw::{avatar, cover, icon, layer, masked, panel, progress, shape, symbol, text};

use std::path::Path;

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

    /// Creates an RGBA color.
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Returns the same color with a different alpha.
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self::rgba(self.red, self.green, self.blue, alpha)
    }

    pub const fn to_argb8888_bytes(self) -> [u8; 4] {
        let red = premultiply_channel(self.red, self.alpha);
        let green = premultiply_channel(self.green, self.alpha);
        let blue = premultiply_channel(self.blue, self.alpha);
        u32::from_be_bytes([self.alpha, red, green, blue]).to_le_bytes()
    }
}

const fn premultiply_channel(channel: u8, alpha: u8) -> u8 {
    ((channel as u16 * alpha as u16 + 127) / 255) as u8
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
    #[error("buffer size mismatch: target {target:?}, overlay {overlay:?}")]
    BufferSizeMismatch {
        target: FrameSize,
        overlay: FrameSize,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
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

/// Borrowed ARGB8888 software buffer view.
pub struct SoftwareBufferView<'a> {
    size: FrameSize,
    pixels: &'a mut [u8],
}

/// Mutable ARGB8888 render target.
pub trait PixelBuffer {
    fn size(&self) -> FrameSize;
    fn pixels(&self) -> &[u8];
    fn pixels_mut(&mut self) -> &mut [u8];

    fn clear(&mut self, color: ClearColor) {
        let pixel = color.to_argb8888_bytes();
        self.pixels_mut()
            .chunks_exact_mut(4)
            .for_each(|chunk| chunk.copy_from_slice(&pixel));
    }

    fn blend_from(&mut self, overlay: &impl PixelBuffer) -> Result<()> {
        if self.size() != overlay.size() {
            return Err(RendererError::BufferSizeMismatch {
                target: self.size(),
                overlay: overlay.size(),
            });
        }

        for (dst, src) in self
            .pixels_mut()
            .chunks_exact_mut(4)
            .zip(overlay.pixels().chunks_exact(4))
        {
            blend_pixel(dst, src);
        }

        Ok(())
    }
}

pub fn copy_rect_from(
    source: &impl PixelBuffer,
    target: &mut impl PixelBuffer,
    rect: shape::Rect,
) -> Result<Option<shape::Rect>> {
    if source.size() != target.size() {
        return Err(RendererError::BufferSizeMismatch {
            target: target.size(),
            overlay: source.size(),
        });
    }

    let size = target.size();
    let rect = rect.clipped_to(size.width as i32, size.height as i32);
    if rect.is_empty() {
        return Ok(None);
    }

    let stride = size.width as usize * 4;
    let left = rect.x as usize * 4;
    let row_len = rect.width as usize * 4;
    let source_pixels = source.pixels();
    let target_pixels = target.pixels_mut();

    for row in rect.y as usize..rect.bottom() as usize {
        let start = row * stride + left;
        let end = start + row_len;
        target_pixels[start..end].copy_from_slice(&source_pixels[start..end]);
    }

    Ok(Some(rect))
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
        PixelBuffer::clear(self, color);
    }

    /// Blends another ARGB8888 buffer over this one.
    pub fn blend_from(&mut self, overlay: &impl PixelBuffer) -> Result<()> {
        PixelBuffer::blend_from(self, overlay)
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

    /// Saves the current ARGB8888 buffer as a PNG image.
    pub fn save_png(&self, path: &Path) -> Result<()> {
        let mut rgba = Vec::with_capacity(self.pixels.len());

        for pixel in self.pixels.chunks_exact(4) {
            let blue = pixel[0];
            let green = pixel[1];
            let red = pixel[2];
            let alpha = pixel[3];

            if alpha == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }

            rgba.extend_from_slice(&[
                unpremultiply_channel(red, alpha),
                unpremultiply_channel(green, alpha),
                unpremultiply_channel(blue, alpha),
                alpha,
            ]);
        }

        let image = image::RgbaImage::from_raw(self.size.width, self.size.height, rgba)
            .ok_or(RendererError::InvalidFrameSize(self.size))?;
        image.save(path)?;
        Ok(())
    }
}

impl PixelBuffer for SoftwareBuffer {
    fn size(&self) -> FrameSize {
        self.size
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
}

impl<'a> SoftwareBufferView<'a> {
    pub fn new(size: FrameSize, pixels: &'a mut [u8]) -> Result<Self> {
        let Some(byte_len) = size.byte_len() else {
            return Err(RendererError::InvalidFrameSize(size));
        };
        if pixels.len() != byte_len {
            return Err(RendererError::InvalidFrameSize(size));
        }

        Ok(Self { size, pixels })
    }
}

impl PixelBuffer for SoftwareBufferView<'_> {
    fn size(&self) -> FrameSize {
        self.size
    }

    fn pixels(&self) -> &[u8] {
        self.pixels
    }

    fn pixels_mut(&mut self) -> &mut [u8] {
        self.pixels
    }
}

fn blend_pixel(dst: &mut [u8], src: &[u8]) {
    let src_alpha = src[3] as u16;
    if src_alpha == 0 {
        return;
    }

    if src_alpha == u16::from(u8::MAX) {
        dst.copy_from_slice(src);
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    for index in 0..4 {
        dst[index] = blend_component(dst[index], src[index], inverse_alpha);
    }
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}

fn unpremultiply_channel(channel: u8, alpha: u8) -> u8 {
    if alpha == 0 {
        return 0;
    }

    ((u32::from(channel) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255) as u8
}

#[cfg(test)]
mod tests {
    use super::{ClearColor, FrameSize, RendererError, SoftwareBuffer, copy_rect_from};
    use crate::shape::Rect;

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
    fn copies_clipped_rect_between_matching_buffers() {
        let source =
            SoftwareBuffer::solid(FrameSize::new(4, 4), ClearColor::opaque(10, 20, 30)).unwrap();
        let mut target = SoftwareBuffer::solid(FrameSize::new(4, 4), ClearColor::opaque(0, 0, 0))
            .expect("target");

        let copied =
            copy_rect_from(&source, &mut target, Rect::new(1, 1, 2, 2)).expect("copy rect");
        assert_eq!(copied, Some(Rect::new(1, 1, 2, 2)));

        let stride = target.size().width as usize;
        for y in 0..4 {
            for x in 0..4 {
                let offset = (y * stride + x) * 4;
                let pixel = &target.pixels()[offset..offset + 4];
                if (1..3).contains(&x) && (1..3).contains(&y) {
                    assert_eq!(pixel, &[30, 20, 10, 255]);
                } else {
                    assert_eq!(pixel, &[0, 0, 0, 255]);
                }
            }
        }
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

    #[test]
    fn blends_translucent_buffers() {
        let mut target =
            SoftwareBuffer::solid(FrameSize::new(1, 1), ClearColor::opaque(255, 128, 0))
                .expect("target");
        let overlay =
            SoftwareBuffer::solid(FrameSize::new(1, 1), ClearColor::rgba(255, 255, 255, 26))
                .expect("overlay");

        target.blend_from(&overlay).expect("blend should succeed");

        assert_eq!(target.pixels(), &[26, 141, 255, 255]);
    }

    #[test]
    fn rejects_mismatched_buffer_sizes() {
        let mut target = SoftwareBuffer::new(FrameSize::new(1, 1)).expect("target");
        let overlay = SoftwareBuffer::new(FrameSize::new(2, 1)).expect("overlay");

        let error = target.blend_from(&overlay).expect_err("blend should fail");

        assert!(matches!(
            error,
            RendererError::BufferSizeMismatch {
                target: FrameSize {
                    width: 1,
                    height: 1
                },
                overlay: FrameSize {
                    width: 2,
                    height: 1
                },
            }
        ));
    }
}
