use std::cell::RefCell;

use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Wrap};

use crate::{ClearColor, ShadowStyle, SoftwareBuffer};

thread_local! {
    static TEXT_ENGINE: RefCell<TextEngine> = RefCell::new(TextEngine::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    pub color: ClearColor,
    pub scale: u32,
    pub letter_spacing: u32,
    pub line_spacing: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    text: String,
    max_width: Option<u32>,
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
        draw_text_internal(
            buffer,
            x,
            y,
            &self.text,
            self.style,
            self.max_width,
            self.style.color,
        );
    }

    /// Draws the laid out text block with a simple drop shadow.
    pub fn draw_with_shadow(
        &self,
        buffer: &mut SoftwareBuffer,
        x: i32,
        y: i32,
        shadow: ShadowStyle,
    ) {
        draw_text_internal(
            buffer,
            x + shadow.offset_x,
            y + shadow.offset_y,
            &self.text,
            self.style,
            self.max_width,
            shadow.color,
        );
        self.draw(buffer, x, y);
    }
}

pub fn measure_text(text: &str, style: TextStyle) -> (u32, u32) {
    measure_text_internal(text, style, None)
}

pub fn draw_text(buffer: &mut SoftwareBuffer, x: i32, y: i32, text: &str, style: TextStyle) {
    draw_text_internal(buffer, x, y, text, style, None, style.color);
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
    draw_text_internal(
        buffer,
        x + shadow.offset_x,
        y + shadow.offset_y,
        text,
        style,
        None,
        shadow.color,
    );
    draw_text(buffer, x, y, text, style);
}

pub fn wrap_text(text: &str, style: TextStyle, max_width: u32) -> TextBlock {
    build_text_block(text, style, Some(max_width))
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

struct TextEngine {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextEngine {
    fn new() -> Self {
        let mut font_system = FontSystem::new();
        #[cfg(target_os = "macos")]
        font_system.db_mut().set_sans_serif_family("Helvetica Neue");
        #[cfg(target_os = "windows")]
        font_system.db_mut().set_sans_serif_family("Segoe UI");
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        font_system.db_mut().set_sans_serif_family("Noto Sans");

        Self {
            font_system,
            swash_cache: SwashCache::new(),
        }
    }
}

fn build_text_block(text: &str, style: TextStyle, max_width: Option<u32>) -> TextBlock {
    let (width, height) = measure_text_internal(text, style, max_width);
    TextBlock {
        text: text.to_string(),
        max_width,
        style,
        width,
        height,
    }
}

fn measure_text_internal(text: &str, style: TextStyle, max_width: Option<u32>) -> (u32, u32) {
    TEXT_ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        let buffer = prepare_buffer(&mut engine.font_system, text, style, max_width);
        let mut width = 0.0_f32;
        let mut height = 0.0_f32;

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
        }

        let fallback_height = text_metrics(style).line_height.ceil() as u32;
        (
            width.ceil() as u32,
            (height.ceil() as u32).max(fallback_height),
        )
    })
}

fn draw_text_internal(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    text: &str,
    style: TextStyle,
    max_width: Option<u32>,
    color: ClearColor,
) {
    TEXT_ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        let TextEngine {
            font_system,
            swash_cache,
        } = &mut *engine;
        let text_buffer = prepare_buffer(font_system, text, style, max_width);
        text_buffer.draw(
            font_system,
            swash_cache,
            cosmic_color(color),
            |left, top, width, height, pixel_color| {
                blend_rect(buffer, x + left, y + top, width, height, pixel_color);
            },
        );
    });
}

fn prepare_buffer(
    font_system: &mut FontSystem,
    text: &str,
    style: TextStyle,
    max_width: Option<u32>,
) -> Buffer {
    let mut buffer = Buffer::new(font_system, text_metrics(style));
    let attrs = text_attrs();
    {
        let mut buffer = buffer.borrow_with(font_system);
        buffer.set_wrap(if max_width.is_some() {
            Wrap::WordOrGlyph
        } else {
            Wrap::None
        });
        buffer.set_size(max_width.map(|width| width as f32), None);
        buffer.set_text(text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(false);
    }
    buffer
}

fn text_attrs() -> Attrs<'static> {
    Attrs::new().family(Family::SansSerif)
}

fn text_metrics(style: TextStyle) -> Metrics {
    let scale = style.scale.max(1) as f32;
    let font_size = 8.0 + scale * 5.0;
    let line_height = font_size + style.line_spacing.max(style.scale) as f32;
    Metrics::new(font_size, line_height)
}

fn cosmic_color(color: ClearColor) -> Color {
    Color::rgba(color.red, color.green, color.blue, color.alpha)
}

fn scale_component(component: u32, current_scale: u32, next_scale: u32) -> u32 {
    let scaled = component
        .saturating_mul(next_scale)
        .div_ceil(current_scale.max(1));
    scaled.max(next_scale.min(1))
}

fn blend_rect(buffer: &mut SoftwareBuffer, x: i32, y: i32, width: u32, height: u32, color: Color) {
    let size = buffer.size();
    let left = x.clamp(0, size.width as i32) as usize;
    let top = y.clamp(0, size.height as i32) as usize;
    let right = (x + width as i32).clamp(0, size.width as i32) as usize;
    let bottom = (y + height as i32).clamp(0, size.height as i32) as usize;

    if left >= right || top >= bottom {
        return;
    }

    let stride = size.width as usize * 4;
    let pixels = buffer.pixels_mut();
    let src_alpha = color.a() as u16;

    for row in top..bottom {
        let row_start = row * stride;
        for column in left..right {
            let offset = row_start + column * 4;
            if src_alpha == u8::MAX as u16 {
                pixels[offset] = color.b();
                pixels[offset + 1] = color.g();
                pixels[offset + 2] = color.r();
                pixels[offset + 3] = color.a();
                continue;
            }

            let inverse_alpha = u16::from(u8::MAX) - src_alpha;
            let dst_alpha = pixels[offset + 3] as u16;

            pixels[offset] = blend_channel(color.b(), pixels[offset], src_alpha, inverse_alpha);
            pixels[offset + 1] =
                blend_channel(color.g(), pixels[offset + 1], src_alpha, inverse_alpha);
            pixels[offset + 2] =
                blend_channel(color.r(), pixels[offset + 2], src_alpha, inverse_alpha);
            pixels[offset + 3] = (src_alpha + (dst_alpha * inverse_alpha) / u16::from(u8::MAX))
                .min(u16::from(u8::MAX)) as u8;
        }
    }
}

fn blend_channel(src: u8, dst: u8, src_alpha: u16, inverse_alpha: u16) -> u8 {
    (((src as u16 * src_alpha) + (dst as u16 * inverse_alpha)) / u16::from(u8::MAX)) as u8
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
        let (width, height) = measure_text("AB", style);

        assert!(width > 0);
        assert!(height > 0);
    }

    #[test]
    fn wraps_text_to_requested_width() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let block = wrap_text("one two three", style, 70);

        assert!(block.width <= 70);
        assert!(block.height > 0);
    }

    #[test]
    fn reduces_scale_to_fit_narrow_widths() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 3);
        let block = fit_wrapped_text("W", style, 10, 1);

        assert!(block.style.scale < 3);
        assert_eq!(block.style.scale, 1);
        assert!(block.width > 0);
    }

    #[test]
    fn renders_non_empty_text() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(96, 48)).expect("buffer");

        draw_text(&mut buffer, 0, 0, "Kwylock", style);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn renders_shadowed_text() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let shadow = ShadowStyle::new(ClearColor::opaque(8, 10, 14), 2, 2);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(96, 48)).expect("buffer");

        draw_text_with_shadow(&mut buffer, 0, 0, "Kwylock", style, shadow);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
