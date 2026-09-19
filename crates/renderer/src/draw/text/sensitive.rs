use cosmic_text::Wrap;
use zeroize::{Zeroize, Zeroizing};

use crate::PixelBuffer;

use super::{
    TextBlock, TextStyle, fitting_ellipsis, layout::layout_text_block,
    raster::draw_sensitive_text_lines,
};

#[derive(Debug)]
pub struct SensitiveTextBlock(TextBlock);

impl SensitiveTextBlock {
    pub const fn height(&self) -> u32 {
        self.0.height
    }

    pub fn draw(&self, buffer: &mut impl PixelBuffer, x: i32, y: i32) {
        draw_sensitive_text_lines(
            buffer,
            x,
            y,
            &self.0.lines,
            self.0.style.clone(),
            self.0.style.color,
        );
    }
}

impl Drop for SensitiveTextBlock {
    fn drop(&mut self) {
        zeroize_text_block(&mut self.0);
    }
}

pub fn fit_sensitive_single_line_text(
    text: &str,
    style: TextStyle,
    max_width: u32,
) -> SensitiveTextBlock {
    let mut block = layout_text_block(text, style.clone(), Some(max_width), Wrap::None);
    if block.width <= max_width && block.lines.len() <= 1 {
        return SensitiveTextBlock(block);
    }
    zeroize_text_block(&mut block);

    let dots = fitting_ellipsis(style.clone(), max_width);
    if dots.is_empty() {
        return SensitiveTextBlock(layout_text_block("", style, Some(max_width), Wrap::None));
    }

    SensitiveTextBlock(fit_truncated_text(text, style, max_width, &dots))
}

fn fit_truncated_text(text: &str, style: TextStyle, max_width: u32, dots: &str) -> TextBlock {
    let chars = Zeroizing::new(text.chars().collect::<Vec<_>>());
    let mut low = 0usize;
    let mut high = chars.len();
    let mut best = Zeroizing::new(dots.to_owned());

    while low <= high {
        let mid = (low + high) / 2;
        let mut candidate = Zeroizing::new(String::with_capacity(text.len() + dots.len()));
        candidate.extend(chars[..mid].iter().copied());
        candidate.push_str(dots);
        let mut block = layout_text_block(&candidate, style.clone(), Some(max_width), Wrap::None);
        let fits = block.width <= max_width && block.lines.len() <= 1;
        zeroize_text_block(&mut block);

        if fits {
            best = candidate;
            low = mid.saturating_add(1);
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }

    layout_text_block(&best, style, Some(max_width), Wrap::None)
}

fn zeroize_text_block(block: &mut TextBlock) {
    for line in &mut block.lines {
        line.zeroize();
    }
    block.lines.clear();
}
