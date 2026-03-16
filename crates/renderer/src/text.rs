use font8x8::{BASIC_FONTS, UnicodeFonts};

use crate::{ClearColor, SoftwareBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    pub color: ClearColor,
    pub scale: u32,
    pub letter_spacing: u32,
    pub line_spacing: u32,
}

impl TextStyle {
    pub const fn new(color: ClearColor, scale: u32) -> Self {
        Self {
            color,
            scale,
            letter_spacing: scale,
            line_spacing: scale * 2,
        }
    }
}

pub fn measure_text(text: &str, style: TextStyle) -> (u32, u32) {
    let scale = style.scale.max(1);
    let glyph_width = 8 * scale;
    let glyph_height = 8 * scale;
    let advance = glyph_width + style.letter_spacing;
    let mut width = 0;
    let mut height = glyph_height;

    for (index, line) in text.lines().enumerate() {
        let line_width = measure_line_width(line, advance, style.letter_spacing);
        width = width.max(line_width);

        if index > 0 {
            height += glyph_height + style.line_spacing;
        }
    }

    (width, height)
}

pub fn draw_text(buffer: &mut SoftwareBuffer, x: i32, y: i32, text: &str, style: TextStyle) {
    let scale = style.scale.max(1) as i32;
    let advance = (8 * style.scale.max(1) + style.letter_spacing) as i32;
    let line_height = (8 * style.scale.max(1) + style.line_spacing) as i32;
    let pixel = style.color.to_argb8888_bytes();

    for (line_index, line) in text.lines().enumerate() {
        let baseline_y = y + line_index as i32 * line_height;
        for (character_index, character) in line.chars().enumerate() {
            let glyph = BASIC_FONTS.get(character).or_else(|| BASIC_FONTS.get('?'));
            let Some(glyph) = glyph else {
                continue;
            };
            let baseline_x = x + character_index as i32 * advance;
            draw_glyph(buffer, baseline_x, baseline_y, scale, &pixel, &glyph);
        }
    }
}

fn measure_line_width(line: &str, advance: u32, letter_spacing: u32) -> u32 {
    let count = line.chars().count() as u32;
    count
        .saturating_mul(advance)
        .saturating_sub(if count > 0 { letter_spacing } else { 0 })
}

fn draw_glyph(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    scale: i32,
    pixel: &[u8; 4],
    glyph: &[u8; 8],
) {
    for (row, bits) in glyph.iter().enumerate() {
        for column in 0..8 {
            if (bits & (1 << column)) == 0 {
                continue;
            }

            fill_scaled_pixel(
                buffer,
                x + column * scale,
                y + row as i32 * scale,
                scale,
                pixel,
            );
        }
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
    use super::{TextStyle, draw_text, measure_text};
    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    #[test]
    fn measures_text_blocks() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        assert_eq!(measure_text("AB", style), (34, 16));
    }

    #[test]
    fn renders_non_empty_text() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(64, 32)).expect("buffer");

        draw_text(&mut buffer, 0, 0, "K", style);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
