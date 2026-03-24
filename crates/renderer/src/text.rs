use font8x8::{BASIC_FONTS, UnicodeFonts};

use crate::{ClearColor, ShadowStyle, SoftwareBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    pub color: ClearColor,
    pub scale: u32,
    pub letter_spacing: u32,
    pub line_spacing: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    pub lines: Vec<String>,
    pub style: TextStyle,
    pub width: u32,
    pub height: u32,
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

    pub fn with_scale(self, scale: u32) -> Self {
        let current_scale = self.scale.max(1);
        let next_scale = scale.max(1);

        Self {
            color: self.color,
            scale: next_scale,
            letter_spacing: scale_component(self.letter_spacing, current_scale, next_scale),
            line_spacing: scale_component(self.line_spacing, current_scale, next_scale),
        }
    }
}

impl TextBlock {
    /// Draws the laid out text block.
    pub fn draw(&self, buffer: &mut SoftwareBuffer, x: i32, y: i32) {
        draw_lines(buffer, x, y, &self.lines, self.style, self.style.color);
    }

    /// Draws the laid out text block with a simple drop shadow.
    pub fn draw_with_shadow(
        &self,
        buffer: &mut SoftwareBuffer,
        x: i32,
        y: i32,
        shadow: ShadowStyle,
    ) {
        draw_lines(
            buffer,
            x + shadow.offset_x,
            y + shadow.offset_y,
            &self.lines,
            self.style,
            shadow.color,
        );
        self.draw(buffer, x, y);
    }
}

pub fn measure_text(text: &str, style: TextStyle) -> (u32, u32) {
    let block = layout_text_lines(text.lines().map(String::from).collect(), style);
    (block.width, block.height)
}

pub fn draw_text(buffer: &mut SoftwareBuffer, x: i32, y: i32, text: &str, style: TextStyle) {
    layout_text_lines(text.lines().map(String::from).collect(), style).draw(buffer, x, y);
}

/// Draws text with a simple drop shadow.
pub fn draw_text_with_shadow(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    text: &str,
    style: TextStyle,
    shadow: ShadowStyle,
) {
    layout_text_lines(text.lines().map(String::from).collect(), style)
        .draw_with_shadow(buffer, x, y, shadow);
}

pub fn wrap_text(text: &str, style: TextStyle, max_width: u32) -> TextBlock {
    let advance = glyph_advance(style);
    let max_chars = max_chars_per_line(max_width, advance, style.letter_spacing);
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }

        wrap_paragraph(paragraph, max_chars, &mut lines);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    layout_text_lines(lines, style)
}

pub fn fit_wrapped_text(text: &str, style: TextStyle, max_width: u32, min_scale: u32) -> TextBlock {
    let preferred_scale = style.scale.max(1);
    let min_scale = min_scale.max(1).min(preferred_scale);

    for scale in (min_scale..=preferred_scale).rev() {
        let block = wrap_text(text, style.with_scale(scale), max_width);
        if block.width <= max_width {
            return block;
        }
    }

    wrap_text(text, style.with_scale(min_scale), max_width)
}

fn layout_text_lines(lines: Vec<String>, style: TextStyle) -> TextBlock {
    let glyph_height = 8 * style.scale.max(1);
    let mut width = 0;
    let mut height = glyph_height;

    for (index, line) in lines.iter().enumerate() {
        width = width.max(measure_line_width(line, style));
        if index > 0 {
            height += glyph_height + style.line_spacing;
        }
    }

    TextBlock {
        lines,
        style,
        width,
        height,
    }
}

fn wrap_paragraph(paragraph: &str, max_chars: usize, lines: &mut Vec<String>) {
    let mut current = String::new();

    for word in paragraph.split_whitespace() {
        for segment in split_word(word, max_chars) {
            if current.is_empty() {
                current.push_str(&segment);
                continue;
            }

            if current.chars().count() + 1 + segment.chars().count() <= max_chars {
                current.push(' ');
                current.push_str(&segment);
            } else {
                lines.push(current);
                current = segment;
            }
        }
    }

    if current.is_empty() {
        lines.push(String::new());
    } else {
        lines.push(current);
    }
}

fn split_word(word: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 || word.chars().count() <= max_chars {
        return vec![word.to_string()];
    }

    let mut segments = Vec::new();
    let mut current = String::new();

    for character in word.chars() {
        current.push(character);
        if current.chars().count() == max_chars {
            segments.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn max_chars_per_line(max_width: u32, advance: u32, letter_spacing: u32) -> usize {
    ((max_width + letter_spacing) / advance.max(1)).max(1) as usize
}

fn glyph_advance(style: TextStyle) -> u32 {
    8 * style.scale.max(1) + style.letter_spacing
}

fn glyph_line_height(style: TextStyle) -> u32 {
    8 * style.scale.max(1) + style.line_spacing
}

fn measure_line_width(line: &str, style: TextStyle) -> u32 {
    let count = line.chars().count() as u32;
    let advance = glyph_advance(style);
    count
        .saturating_mul(advance)
        .saturating_sub(if count > 0 { style.letter_spacing } else { 0 })
}

fn scale_component(component: u32, current_scale: u32, next_scale: u32) -> u32 {
    let scaled = component
        .saturating_mul(next_scale)
        .div_ceil(current_scale.max(1));
    scaled.max(next_scale.min(1))
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

fn draw_lines(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    lines: &[String],
    style: TextStyle,
    color: ClearColor,
) {
    let scale = style.scale.max(1) as i32;
    let advance = glyph_advance(style) as i32;
    let line_height = glyph_line_height(style) as i32;
    let pixel = color.to_argb8888_bytes();

    for (line_index, line) in lines.iter().enumerate() {
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
    use super::{
        TextStyle, draw_text, draw_text_with_shadow, fit_wrapped_text, measure_text, wrap_text,
    };
    use crate::{ClearColor, FrameSize, ShadowStyle, SoftwareBuffer};

    #[test]
    fn measures_text_blocks() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        assert_eq!(measure_text("AB", style), (34, 16));
    }

    #[test]
    fn wraps_text_to_requested_width() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let block = wrap_text("one two three", style, 70);

        assert!(block.lines.len() > 1);
        assert!(block.width <= 70);
    }

    #[test]
    fn reduces_scale_to_fit_narrow_widths() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 3);
        let block = fit_wrapped_text("W", style, 10, 1);

        assert!(block.style.scale < 3);
        assert!(block.width <= 10);
    }

    #[test]
    fn renders_non_empty_text() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(64, 32)).expect("buffer");

        draw_text(&mut buffer, 0, 0, "K", style);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn renders_shadowed_text() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let shadow = ShadowStyle::new(ClearColor::opaque(8, 10, 14), 2, 2);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(64, 32)).expect("buffer");

        draw_text_with_shadow(&mut buffer, 0, 0, "K", style, shadow);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
