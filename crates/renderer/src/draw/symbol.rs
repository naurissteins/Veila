use crate::{ClearColor, ShadowStyle, SoftwareBuffer};

/// Small reusable bitmap symbols for lock UI state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Info,
    Pending,
    Error,
}

/// Rendering parameters for a bitmap symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolStyle {
    pub color: ClearColor,
    pub scale: u32,
}

impl SymbolStyle {
    /// Creates a new symbol style.
    pub const fn new(color: ClearColor, scale: u32) -> Self {
        Self { color, scale }
    }
}

/// Returns the pixel dimensions for a symbol.
pub fn measure_symbol(style: SymbolStyle) -> (u32, u32) {
    let scale = style.scale.max(1);
    (8 * scale, 8 * scale)
}

/// Draws a symbol into an ARGB8888 software buffer.
pub fn draw_symbol(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    symbol: SymbolKind,
    style: SymbolStyle,
) {
    draw_symbol_colored(buffer, x, y, symbol, style.scale.max(1) as i32, style.color);
}

/// Draws a symbol with a simple drop shadow.
pub fn draw_symbol_with_shadow(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    symbol: SymbolKind,
    style: SymbolStyle,
    shadow: ShadowStyle,
) {
    draw_symbol_colored(
        buffer,
        x + shadow.offset_x,
        y + shadow.offset_y,
        symbol,
        style.scale.max(1) as i32,
        shadow.color,
    );
    draw_symbol(buffer, x, y, symbol, style);
}

fn draw_symbol_colored(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    symbol: SymbolKind,
    scale: i32,
    color: ClearColor,
) {
    let pixel = color.to_argb8888_bytes();

    for (row, bits) in bitmap(symbol).iter().enumerate() {
        for column in 0..8 {
            if (bits & (1 << column)) == 0 {
                continue;
            }

            fill_scaled_pixel(
                buffer,
                x + column * scale,
                y + row as i32 * scale,
                scale,
                &pixel,
            );
        }
    }
}

fn bitmap(symbol: SymbolKind) -> &'static [u8; 8] {
    match symbol {
        SymbolKind::Info => &[
            0b00011000, 0b00011000, 0b00000000, 0b00011000, 0b00011000, 0b00011000, 0b00011000,
            0b00000000,
        ],
        SymbolKind::Pending => &[
            0b00111100, 0b01000010, 0b10000001, 0b10011001, 0b10001001, 0b10000001, 0b01000010,
            0b00111100,
        ],
        SymbolKind::Error => &[
            0b10000001, 0b01000010, 0b00100100, 0b00011000, 0b00011000, 0b00100100, 0b01000010,
            0b10000001,
        ],
    }
}

fn fill_scaled_pixel(buffer: &mut SoftwareBuffer, x: i32, y: i32, scale: i32, pixel: &[u8; 4]) {
    let size = buffer.size();
    let left = x.clamp(0, size.width as i32);
    let top = y.clamp(0, size.height as i32);
    let right = (x + scale).clamp(0, size.width as i32);
    let bottom = (y + scale).clamp(0, size.height as i32);

    if left >= right || top >= bottom {
        return;
    }

    let stride = size.width as usize * 4;
    let pixels = buffer.pixels_mut();

    for row in top as usize..bottom as usize {
        let row_start = row * stride;
        for column in left as usize..right as usize {
            let offset = row_start + column * 4;
            pixels[offset..offset + 4].copy_from_slice(pixel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SymbolKind, SymbolStyle, draw_symbol, draw_symbol_with_shadow, measure_symbol};
    use crate::{ClearColor, FrameSize, ShadowStyle, SoftwareBuffer};

    #[test]
    fn measures_scaled_symbols() {
        assert_eq!(
            measure_symbol(SymbolStyle::new(ClearColor::opaque(255, 255, 255), 2)),
            (16, 16)
        );
    }

    #[test]
    fn renders_non_empty_symbols() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(24, 24)).expect("buffer");
        draw_symbol(
            &mut buffer,
            4,
            4,
            SymbolKind::Pending,
            SymbolStyle::new(ClearColor::opaque(255, 194, 92), 2),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn symbol_variants_have_distinct_bitmaps() {
        let mut pending = SoftwareBuffer::new(FrameSize::new(16, 16)).expect("buffer");
        let mut error = SoftwareBuffer::new(FrameSize::new(16, 16)).expect("buffer");

        draw_symbol(
            &mut pending,
            0,
            0,
            SymbolKind::Pending,
            SymbolStyle::new(ClearColor::opaque(255, 255, 255), 1),
        );
        draw_symbol(
            &mut error,
            0,
            0,
            SymbolKind::Error,
            SymbolStyle::new(ClearColor::opaque(255, 255, 255), 1),
        );

        assert_ne!(pending.pixels(), error.pixels());
    }

    #[test]
    fn renders_shadowed_symbols() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(24, 24)).expect("buffer");
        draw_symbol_with_shadow(
            &mut buffer,
            4,
            4,
            SymbolKind::Error,
            SymbolStyle::new(ClearColor::opaque(220, 96, 96), 2),
            ShadowStyle::new(ClearColor::opaque(8, 10, 14), 2, 2),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
