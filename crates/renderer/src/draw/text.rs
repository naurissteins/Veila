use std::{cell::RefCell, thread_local};

use cosmic_text::{
    Attrs, Buffer, Color, Family, FamilyOwned, FontSystem, Metrics, Shaping, SwashCache, Wrap,
};

use crate::{ClearColor, ShadowStyle, SoftwareBuffer};

const BUNDLED_CLOCK_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/prototype.regular.ttf"
));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStyle {
    pub color: ClearColor,
    pub scale: u32,
    pub letter_spacing: u32,
    pub line_spacing: u32,
    pub font_family: Option<FamilyOwned>,
    pub font_weight: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    pub lines: Vec<String>,
    pub style: TextStyle,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
struct FontContext {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

thread_local! {
    static FONT_CONTEXT: RefCell<FontContext> = RefCell::new(FontContext {
        font_system: {
            let mut font_system = FontSystem::new();
            font_system.db_mut().load_font_data(BUNDLED_CLOCK_FONT.to_vec());
            font_system
        },
        swash_cache: SwashCache::new(),
    });
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
        }
    }

    pub fn with_line_spacing(mut self, line_spacing: u32) -> Self {
        self.line_spacing = line_spacing;
        self
    }

    pub fn with_font_family(mut self, family: &str) -> Self {
        let trimmed = family.trim();
        if !trimmed.is_empty() {
            self.font_family = Some(FamilyOwned::new(Family::Name(trimmed)));
        }
        self
    }

    pub fn with_font_weight(mut self, weight: u16) -> Self {
        self.font_weight = Some(weight);
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
    let block = layout_text_block(text, style, None, Wrap::None);
    (block.width, block.height)
}

pub fn draw_text(buffer: &mut SoftwareBuffer, x: i32, y: i32, text: &str, style: TextStyle) {
    layout_text_block(text, style, None, Wrap::None).draw(buffer, x, y);
}

pub fn draw_text_with_shadow(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    text: &str,
    style: TextStyle,
    shadow: ShadowStyle,
) {
    layout_text_block(text, style, None, Wrap::None).draw_with_shadow(buffer, x, y, shadow);
}

pub fn wrap_text(text: &str, style: TextStyle, max_width: u32) -> TextBlock {
    layout_text_block(text, style, Some(max_width), Wrap::WordOrGlyph)
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

pub fn bundled_clock_font_family() -> Option<String> {
    FONT_CONTEXT.with(|context| {
        let context = context.borrow();
        context
            .font_system
            .db()
            .faces()
            .find(|face| matches!(&face.source, cosmic_text::fontdb::Source::Binary(_)))
            .and_then(|face| face.families.first().map(|(family, _)| family.clone()))
    })
}

pub fn bundled_clock_font_postscript_name() -> Option<String> {
    FONT_CONTEXT.with(|context| {
        let context = context.borrow();
        context
            .font_system
            .db()
            .faces()
            .find(|face| matches!(&face.source, cosmic_text::fontdb::Source::Binary(_)))
            .map(|face| face.post_script_name.clone())
    })
}

pub fn resolve_font_family(requested: &str) -> Option<String> {
    let requested = requested.trim();
    if requested.is_empty() {
        return None;
    }

    FONT_CONTEXT.with(|context| {
        let context = context.borrow();
        resolve_font_family_in_db(context.font_system.db(), requested)
    })
}

fn layout_text_block(
    text: &str,
    style: TextStyle,
    max_width: Option<u32>,
    wrap: Wrap,
) -> TextBlock {
    if text.is_empty() {
        let height = line_height(&style);
        return TextBlock {
            lines: vec![String::new()],
            style,
            width: 0,
            height,
        };
    }

    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let metrics = Metrics::new(font_size(&style), line_height(&style) as f32);
        let mut buffer = Buffer::new(&mut context.font_system, metrics);
        buffer.set_wrap(&mut context.font_system, wrap);
        buffer.set_size(
            &mut context.font_system,
            max_width.map(|value| value as f32),
            None,
        );
        let attrs = text_attrs(&style);
        buffer.set_text(&mut context.font_system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut context.font_system, true);

        let mut width = 0.0f32;
        let mut bottom = 0.0f32;
        let mut lines = Vec::new();

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            bottom = bottom.max(run.line_top + run.line_height);
            lines.push(extract_run_text(run.text, run.glyphs));
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        let height = bottom.ceil().max(line_height(&style) as f32) as u32;

        TextBlock {
            lines,
            style,
            width: width.ceil().max(0.0) as u32,
            height,
        }
    })
}

fn resolve_font_family_in_db(
    db: &cosmic_text::fontdb::Database,
    requested: &str,
) -> Option<String> {
    let requested = normalize_font_name(requested);
    let mut partial_match = None;

    for face in db.faces() {
        for (family, _) in &face.families {
            let normalized_family = normalize_font_name(family);
            if normalized_family == requested {
                return Some(family.clone());
            }

            if partial_match.is_none()
                && (normalized_family.contains(&requested)
                    || requested.contains(&normalized_family))
            {
                partial_match = Some(family.clone());
            }
        }

        if normalize_font_name(&face.post_script_name) == requested
            && let Some((family, _)) = face.families.first()
        {
            return Some(family.clone());
        }
    }

    partial_match
}

fn normalize_font_name(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn draw_text_lines(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    lines: &[String],
    style: TextStyle,
    color: ClearColor,
) {
    if lines.iter().all(String::is_empty) {
        return;
    }

    let text = lines.join("\n");
    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let FontContext {
            font_system,
            swash_cache,
        } = &mut *context;
        let metrics = Metrics::new(font_size(&style), line_height(&style) as f32);
        let mut cosmic_buffer = Buffer::new(font_system, metrics);
        cosmic_buffer.set_wrap(font_system, Wrap::None);
        cosmic_buffer.set_size(font_system, None, None);
        let attrs = text_attrs(&style);
        cosmic_buffer.set_text(font_system, &text, &attrs, Shaping::Advanced);
        cosmic_buffer.shape_until_scroll(font_system, true);

        cosmic_buffer.draw(
            font_system,
            swash_cache,
            text_color(color),
            |pixel_x, pixel_y, width, height, pixel_color| {
                let pixel_color = modulate_alpha(pixel_color, color.alpha);
                for offset_y in 0..height as i32 {
                    for offset_x in 0..width as i32 {
                        blend_pixel(
                            buffer,
                            x + pixel_x + offset_x,
                            y + pixel_y + offset_y,
                            pixel_color,
                        );
                    }
                }
            },
        );
    });
}

fn extract_run_text(text: &str, glyphs: &[cosmic_text::LayoutGlyph]) -> String {
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

fn text_attrs(style: &TextStyle) -> Attrs<'_> {
    let attrs = match style.font_family.as_ref() {
        Some(family) => Attrs::new().family(family.as_family()),
        None => Attrs::new().family(Family::SansSerif),
    };

    match style.font_weight {
        Some(weight) => attrs.weight(cosmic_text::Weight(weight)),
        None => attrs,
    }
}

fn font_size(style: &TextStyle) -> f32 {
    4.0 + style.scale.max(1) as f32 * 6.0
}

fn line_height(style: &TextStyle) -> u32 {
    font_size(style).ceil() as u32 + style.line_spacing
}

fn scale_component(component: u32, current_scale: u32, next_scale: u32) -> u32 {
    let scaled = component
        .saturating_mul(next_scale)
        .div_ceil(current_scale.max(1));
    scaled.max(next_scale.min(1))
}

fn text_color(color: ClearColor) -> Color {
    Color::rgba(color.red, color.green, color.blue, color.alpha)
}

fn modulate_alpha(color: Color, alpha: u8) -> Color {
    let modulated_alpha = ((u16::from(color.a()) * u16::from(alpha) + 127) / 255) as u8;
    Color::rgba(color.r(), color.g(), color.b(), modulated_alpha)
}

fn blend_pixel(buffer: &mut SoftwareBuffer, x: i32, y: i32, color: Color) {
    let size = buffer.size();
    if x < 0 || y < 0 || x >= size.width as i32 || y >= size.height as i32 {
        return;
    }

    let src_alpha = color.a() as u16;
    if src_alpha == 0 {
        return;
    }

    let src_red = premultiply(color.r(), color.a());
    let src_green = premultiply(color.g(), color.a());
    let src_blue = premultiply(color.b(), color.a());
    let stride = size.width as usize * 4;
    let offset = y as usize * stride + x as usize * 4;
    let pixels = buffer.pixels_mut();

    if src_alpha == u16::from(u8::MAX) {
        pixels[offset] = src_blue;
        pixels[offset + 1] = src_green;
        pixels[offset + 2] = src_red;
        pixels[offset + 3] = color.a();
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    pixels[offset] = blend_component(pixels[offset], src_blue, inverse_alpha);
    pixels[offset + 1] = blend_component(pixels[offset + 1], src_green, inverse_alpha);
    pixels[offset + 2] = blend_component(pixels[offset + 2], src_red, inverse_alpha);
    pixels[offset + 3] = blend_component(pixels[offset + 3], color.a(), inverse_alpha);
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::{
        TextStyle, bundled_clock_font_family, bundled_clock_font_postscript_name, draw_text,
        draw_text_with_shadow, fit_wrapped_text, measure_text, resolve_font_family, wrap_text,
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
    fn respects_text_alpha_when_rendering() {
        let mut faint = SoftwareBuffer::solid(FrameSize::new(64, 32), ClearColor::opaque(0, 0, 0))
            .expect("buffer");
        let mut opaque = SoftwareBuffer::solid(FrameSize::new(64, 32), ClearColor::opaque(0, 0, 0))
            .expect("buffer");

        draw_text(
            &mut faint,
            0,
            0,
            "88:88",
            TextStyle::new(ClearColor::rgba(255, 255, 255, 5), 3),
        );
        draw_text(
            &mut opaque,
            0,
            0,
            "88:88",
            TextStyle::new(ClearColor::opaque(255, 255, 255), 3),
        );

        let faint_total: u64 = faint
            .pixels()
            .chunks_exact(4)
            .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
            .sum();
        let opaque_total: u64 = opaque
            .pixels()
            .chunks_exact(4)
            .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
            .sum();

        assert!(faint_total > 0);
        assert!(faint_total < opaque_total);
    }

    #[test]
    fn renders_shadowed_text() {
        let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 2);
        let shadow = ShadowStyle::new(ClearColor::opaque(8, 10, 14), 2, 2);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(64, 32)).expect("buffer");

        draw_text_with_shadow(&mut buffer, 0, 0, "K", style, shadow);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn resolves_bundled_clock_font_family_from_loaded_database() {
        assert!(bundled_clock_font_family().is_some());
    }

    #[test]
    fn resolves_postscript_font_name_to_family_name() {
        let family = bundled_clock_font_family().expect("bundled family");
        let postscript = bundled_clock_font_postscript_name().expect("bundled postscript name");

        assert_eq!(
            resolve_font_family(&postscript).as_deref(),
            Some(family.as_str())
        );
    }
}
