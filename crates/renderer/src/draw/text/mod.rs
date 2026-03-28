mod context;
mod layout;
mod raster;

#[cfg(test)]
mod tests;

use cosmic_text::{FamilyOwned, Style as CosmicFontStyle};

use crate::{ClearColor, ShadowStyle, SoftwareBuffer};

pub use context::{
    bundled_clock_font_family, bundled_clock_font_postscript_name, resolve_font_family,
};
use layout::{font_size, layout_text_block, line_height, scale_component};
use raster::draw_text_lines;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStyle {
    pub color: ClearColor,
    pub scale: u32,
    pub letter_spacing: u32,
    pub line_spacing: u32,
    pub font_family: Option<FamilyOwned>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
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
            letter_spacing: 0,
            line_spacing: scale * 3,
            font_family: None,
            font_weight: None,
            font_style: None,
        }
    }

    pub fn with_scale(&self, scale: u32) -> Self {
        let current_scale = self.scale.max(1);
        let next_scale = scale.max(1);

        Self {
            color: self.color,
            scale: next_scale,
            letter_spacing: scale_component(self.letter_spacing, current_scale, next_scale),
            line_spacing: scale_component(self.line_spacing, current_scale, next_scale),
            font_family: self.font_family.clone(),
            font_weight: self.font_weight,
            font_style: self.font_style,
        }
    }

    pub fn with_line_spacing(mut self, line_spacing: u32) -> Self {
        self.line_spacing = line_spacing;
        self
    }

    pub fn with_letter_spacing(mut self, letter_spacing: u32) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }

    pub fn with_font_family(mut self, family: &str) -> Self {
        let trimmed = family.trim();
        if !trimmed.is_empty() {
            self.font_family = Some(FamilyOwned::new(cosmic_text::Family::Name(trimmed)));
        }
        self
    }

    pub fn with_font_weight(mut self, weight: u16) -> Self {
        self.font_weight = Some(weight);
        self
    }

    pub fn with_font_style(mut self, font_style: FontStyle) -> Self {
        self.font_style = Some(font_style);
        self
    }
}

impl TextBlock {
    pub fn draw(&self, buffer: &mut SoftwareBuffer, x: i32, y: i32) {
        draw_text_lines(
            buffer,
            x,
            y,
            &self.lines,
            self.style.clone(),
            self.style.color,
        );
    }

    pub fn draw_with_shadow(
        &self,
        buffer: &mut SoftwareBuffer,
        x: i32,
        y: i32,
        shadow: ShadowStyle,
    ) {
        draw_text_lines(
            buffer,
            x + shadow.offset_x,
            y + shadow.offset_y,
            &self.lines,
            self.style.clone(),
            shadow.color,
        );
        self.draw(buffer, x, y);
    }
}

pub fn measure_text(text: &str, style: TextStyle) -> (u32, u32) {
    let block = layout_text_block(text, style, None, cosmic_text::Wrap::None);
    (block.width, block.height)
}

pub fn draw_text(buffer: &mut SoftwareBuffer, x: i32, y: i32, text: &str, style: TextStyle) {
    layout_text_block(text, style, None, cosmic_text::Wrap::None).draw(buffer, x, y);
}

pub fn draw_text_with_shadow(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    text: &str,
    style: TextStyle,
    shadow: ShadowStyle,
) {
    layout_text_block(text, style, None, cosmic_text::Wrap::None)
        .draw_with_shadow(buffer, x, y, shadow);
}

pub fn wrap_text(text: &str, style: TextStyle, max_width: u32) -> TextBlock {
    layout_text_block(text, style, Some(max_width), cosmic_text::Wrap::WordOrGlyph)
}

pub fn fit_single_line_text(text: &str, style: TextStyle, max_width: u32) -> TextBlock {
    let block = layout_text_block(
        text,
        style.clone(),
        Some(max_width),
        cosmic_text::Wrap::None,
    );
    if block.width <= max_width && block.lines.len() <= 1 {
        return block;
    }

    let dots = fitting_ellipsis(style.clone(), max_width);
    if dots.is_empty() {
        return layout_text_block("", style, Some(max_width), cosmic_text::Wrap::None);
    }

    let chars: Vec<char> = text.chars().collect();
    let mut low = 0usize;
    let mut high = chars.len();
    let mut best = dots.clone();

    while low <= high {
        let mid = (low + high) / 2;
        let candidate = format!("{}{}", chars[..mid].iter().collect::<String>(), dots);
        let block = layout_text_block(
            &candidate,
            style.clone(),
            Some(max_width),
            cosmic_text::Wrap::None,
        );

        if block.width <= max_width && block.lines.len() <= 1 {
            best = candidate;
            low = mid.saturating_add(1);
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }

    layout_text_block(&best, style, Some(max_width), cosmic_text::Wrap::None)
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

fn fitting_ellipsis(style: TextStyle, max_width: u32) -> String {
    for dots in ["...", "..", "."] {
        let block = layout_text_block(
            dots,
            style.clone(),
            Some(max_width),
            cosmic_text::Wrap::None,
        );
        if block.width <= max_width && block.lines.len() <= 1 {
            return dots.to_owned();
        }
    }

    String::new()
}

pub(super) fn text_attrs(style: &TextStyle) -> cosmic_text::Attrs<'_> {
    let attrs = match style.font_family.as_ref() {
        Some(family) => cosmic_text::Attrs::new().family(family.as_family()),
        None => cosmic_text::Attrs::new().family(cosmic_text::Family::SansSerif),
    };

    let attrs = match style.font_weight {
        Some(weight) => attrs.weight(cosmic_text::Weight(weight)),
        None => attrs,
    };

    let attrs = match style.font_style {
        Some(FontStyle::Normal) => attrs.style(CosmicFontStyle::Normal),
        Some(FontStyle::Italic) => attrs.style(CosmicFontStyle::Italic),
        None => attrs,
    };

    if style.letter_spacing == 0 {
        attrs
    } else {
        attrs.letter_spacing(style.letter_spacing as f32 / font_size(style).max(1.0))
    }
}

pub(super) fn extract_run_text(text: &str, glyphs: &[cosmic_text::LayoutGlyph]) -> String {
    let Some(start) = glyphs.iter().map(|glyph| glyph.start).min() else {
        return String::new();
    };
    let Some(end) = glyphs.iter().map(|glyph| glyph.end).max() else {
        return String::new();
    };

    if start >= end || end > text.len() {
        return String::new();
    }

    text[start..end].to_string()
}

pub(super) fn text_color(color: ClearColor) -> cosmic_text::Color {
    cosmic_text::Color::rgba(color.red, color.green, color.blue, color.alpha)
}

pub(super) fn modulate_alpha(color: cosmic_text::Color, alpha: u8) -> cosmic_text::Color {
    let modulated_alpha = ((u16::from(color.a()) * u16::from(alpha) + 127) / 255) as u8;
    cosmic_text::Color::rgba(color.r(), color.g(), color.b(), modulated_alpha)
}

pub(super) fn font_metrics(style: &TextStyle) -> cosmic_text::Metrics {
    cosmic_text::Metrics::new(font_size(style), line_height(style) as f32)
}
