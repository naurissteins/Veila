use std::{cell::RefCell, sync::OnceLock, thread_local};

use tiny_skia::{FillRule, Paint, Path, PathBuilder, Transform};

use super::skia::color as skia_color;
use crate::{ClearColor, SoftwareBuffer, shape::Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetIcon {
    Eye,
    EyeOff,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedRasterIcon {
    key: IconRasterKey,
    pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IconRasterKey {
    icon: AssetIcon,
    width: u32,
    height: u32,
    color: ClearColor,
    padding: i32,
}

#[derive(Debug)]
struct ParsedIcon {
    path: Path,
    viewbox: ViewBox,
}

#[derive(Debug, Clone, Copy)]
struct ViewBox {
    width: f32,
    height: f32,
}

thread_local! {
    static ICON_RASTER_CACHE: RefCell<Vec<CachedRasterIcon>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconStyle {
    pub color: ClearColor,
    pub padding: i32,
}

impl IconStyle {
    pub const fn new(color: ClearColor) -> Self {
        Self { color, padding: 3 }
    }

    pub const fn with_padding(self, padding: i32) -> Self {
        Self {
            color: self.color,
            padding,
        }
    }
}

pub fn draw_icon(buffer: &mut SoftwareBuffer, rect: Rect, icon: AssetIcon, style: IconStyle) {
    if rect.is_empty() {
        return;
    }

    let width = rect.width.max(1) as u32;
    let height = rect.height.max(1) as u32;
    let key = IconRasterKey {
        icon,
        width,
        height,
        color: style.color,
        padding: style.padding,
    };

    ICON_RASTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache
            .iter()
            .position(|entry| entry.key == key)
            .unwrap_or_else(|| {
                cache.push(CachedRasterIcon {
                    key,
                    pixels: rasterize_icon(key),
                });
                cache.len() - 1
            });
        let raster = &cache[index];
        blend_icon_raster(
            buffer,
            rect.x,
            rect.y,
            raster.key.width,
            raster.key.height,
            &raster.pixels,
        );
    });
}

fn rasterize_icon(key: IconRasterKey) -> Vec<u8> {
    let Some(mut pixmap) = tiny_skia::Pixmap::new(key.width, key.height) else {
        return Vec::new();
    };
    let parsed = match key.icon {
        AssetIcon::Eye => eye_icon(),
        AssetIcon::EyeOff => eye_off_icon(),
        AssetIcon::User => user_icon(),
    };
    let inset = key.padding.max(0) as f32;
    let target_width = (key.width as f32 - inset * 2.0).max(1.0);
    let target_height = (key.height as f32 - inset * 2.0).max(1.0);
    let scale = (target_width / parsed.viewbox.width).min(target_height / parsed.viewbox.height);
    let icon_width = parsed.viewbox.width * scale;
    let icon_height = parsed.viewbox.height * scale;
    let translate_x = ((key.width as f32 - icon_width) / 2.0).max(0.0);
    let translate_y = ((key.height as f32 - icon_height) / 2.0).max(0.0);
    let transform = Transform::from_scale(scale, scale).post_translate(translate_x, translate_y);
    let mut paint = Paint::default();
    paint.set_color(skia_color(key.color));
    paint.anti_alias = true;
    pixmap.fill_path(&parsed.path, &paint, FillRule::Winding, transform, None);
    pixmap.take()
}

fn blend_icon_raster(
    buffer: &mut SoftwareBuffer,
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    if pixels.is_empty() || width == 0 || height == 0 {
        return;
    }

    let target_width = buffer.size().width as i32;
    let target_height = buffer.size().height as i32;
    let overlay_width = width as i32;
    let overlay_height = height as i32;

    let left = origin_x.clamp(0, target_width);
    let top = origin_y.clamp(0, target_height);
    let right = (origin_x + overlay_width).clamp(0, target_width);
    let bottom = (origin_y + overlay_height).clamp(0, target_height);

    if left >= right || top >= bottom {
        return;
    }

    let overlay_stride = width as usize * 4;
    let buffer_stride = buffer.size().width as usize * 4;
    let target_pixels = buffer.pixels_mut();

    for y in top..bottom {
        let overlay_y = (y - origin_y) as usize;
        let buffer_y = y as usize;
        for x in left..right {
            let overlay_x = (x - origin_x) as usize;
            let buffer_x = x as usize;
            let src_offset = overlay_y * overlay_stride + overlay_x * 4;
            let dst_offset = buffer_y * buffer_stride + buffer_x * 4;
            blend_pixel(
                &mut target_pixels[dst_offset..dst_offset + 4],
                &pixels[src_offset..src_offset + 4],
            );
        }
    }
}

fn blend_pixel(dst: &mut [u8], src: &[u8]) {
    let src_alpha = src[3] as u16;
    if src_alpha == 0 {
        return;
    }

    if src_alpha == u16::from(u8::MAX) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = src[3];
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    dst[0] = blend_component(dst[0], src[2], inverse_alpha);
    dst[1] = blend_component(dst[1], src[1], inverse_alpha);
    dst[2] = blend_component(dst[2], src[0], inverse_alpha);
    dst[3] = blend_component(dst[3], src[3], inverse_alpha);
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}

fn eye_icon() -> &'static ParsedIcon {
    static ICON: OnceLock<ParsedIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        parse_svg_asset(include_str!("../../../../assets/icons/eye-solid-full.svg"))
    })
}

fn eye_off_icon() -> &'static ParsedIcon {
    static ICON: OnceLock<ParsedIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        parse_svg_asset(include_str!(
            "../../../../assets/icons/eye-slash-solid-full.svg"
        ))
    })
}

fn user_icon() -> &'static ParsedIcon {
    static ICON: OnceLock<ParsedIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        parse_svg_asset(include_str!(
            "../../../../assets/icons/user-regular-full.svg"
        ))
    })
}

fn parse_svg_asset(svg: &str) -> ParsedIcon {
    let data = extract_path_data(svg).unwrap_or_default();
    let viewbox = extract_viewbox(svg).unwrap_or(ViewBox {
        width: 640.0,
        height: 640.0,
    });

    ParsedIcon {
        path: parse_path_data(data).unwrap_or_else(empty_path),
        viewbox,
    }
}

fn extract_path_data(svg: &str) -> Option<&str> {
    let path_start = svg.find("<path")?;
    let data_start = svg[path_start..].find("d=\"")? + path_start + 3;
    let data_end = svg[data_start..].find('"')? + data_start;
    Some(&svg[data_start..data_end])
}

fn extract_viewbox(svg: &str) -> Option<ViewBox> {
    let viewbox_start = svg.find("viewBox=\"")? + 9;
    let viewbox_end = svg[viewbox_start..].find('"')? + viewbox_start;
    let mut parts = svg[viewbox_start..viewbox_end].split_ascii_whitespace();
    let _min_x: f32 = parts.next()?.parse().ok()?;
    let _min_y: f32 = parts.next()?.parse().ok()?;
    let width: f32 = parts.next()?.parse().ok()?;
    let height: f32 = parts.next()?.parse().ok()?;

    Some(ViewBox { width, height })
}

fn parse_path_data(data: &str) -> Option<Path> {
    let mut parser = PathParser::new(data);
    let mut builder = PathBuilder::new();
    let mut command = None;

    while parser.skip_separators() {
        if let Some(next_command) = parser.consume_command() {
            command = Some(next_command);
        }

        match command? {
            'M' => {
                let x = parser.parse_number()?;
                let y = parser.parse_number()?;
                builder.move_to(x, y);
                command = Some('L');
            }
            'L' => {
                let x = parser.parse_number()?;
                let y = parser.parse_number()?;
                builder.line_to(x, y);
            }
            'C' => {
                let x1 = parser.parse_number()?;
                let y1 = parser.parse_number()?;
                let x2 = parser.parse_number()?;
                let y2 = parser.parse_number()?;
                let x = parser.parse_number()?;
                let y = parser.parse_number()?;
                builder.cubic_to(x1, y1, x2, y2, x, y);
            }
            'Z' | 'z' => {
                builder.close();
                command = None;
            }
            _ => return None,
        }
    }

    builder.finish()
}

fn empty_path() -> Path {
    PathBuilder::from_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).expect("rect"))
}

struct PathParser<'a> {
    data: &'a str,
    index: usize,
}

impl<'a> PathParser<'a> {
    fn new(data: &'a str) -> Self {
        Self { data, index: 0 }
    }

    fn skip_separators(&mut self) -> bool {
        while let Some(character) = self.peek() {
            if character.is_ascii_whitespace() || character == ',' {
                self.index += character.len_utf8();
            } else {
                break;
            }
        }

        self.index < self.data.len()
    }

    fn consume_command(&mut self) -> Option<char> {
        let character = self.peek()?;
        if character.is_ascii_alphabetic() {
            self.index += character.len_utf8();
            Some(character)
        } else {
            None
        }
    }

    fn parse_number(&mut self) -> Option<f32> {
        self.skip_separators();
        let start = self.index;
        let mut seen_digit = false;

        while let Some(character) = self.peek() {
            let valid = match character {
                '+' | '-' => self.index == start,
                '.' => true,
                '0'..='9' => {
                    seen_digit = true;
                    true
                }
                _ => false,
            };

            if !valid {
                break;
            }

            self.index += character.len_utf8();
        }

        if !seen_digit || start == self.index {
            return None;
        }

        self.data[start..self.index].parse().ok()
    }

    fn peek(&self) -> Option<char> {
        self.data[self.index..].chars().next()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AssetIcon, ICON_RASTER_CACHE, IconStyle, draw_icon, extract_path_data, extract_viewbox,
        parse_path_data,
    };
    use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

    #[test]
    fn extracts_svg_path_data() {
        let data = extract_path_data(include_str!("../../../../assets/icons/eye-solid-full.svg"));

        assert!(data.is_some());
    }

    #[test]
    fn parses_svg_path_data() {
        let data = extract_path_data(include_str!("../../../../assets/icons/eye-solid-full.svg"))
            .expect("path data");

        assert!(parse_path_data(data).is_some());
    }

    #[test]
    fn extracts_svg_viewbox() {
        let viewbox = extract_viewbox(include_str!("../../../../assets/icons/eye-solid-full.svg"))
            .expect("viewbox");

        assert_eq!(viewbox.width, 640.0);
        assert_eq!(viewbox.height, 640.0);
    }

    #[test]
    fn renders_vector_eye_icon() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");
        draw_icon(
            &mut buffer,
            Rect::new(0, 0, 32, 32),
            AssetIcon::Eye,
            IconStyle::new(ClearColor::opaque(255, 255, 255)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn vector_icons_are_distinct() {
        let mut eye = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");
        let mut eye_off = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");

        draw_icon(
            &mut eye,
            Rect::new(0, 0, 32, 32),
            AssetIcon::Eye,
            IconStyle::new(ClearColor::opaque(255, 255, 255)),
        );
        draw_icon(
            &mut eye_off,
            Rect::new(0, 0, 32, 32),
            AssetIcon::EyeOff,
            IconStyle::new(ClearColor::opaque(255, 255, 255)),
        );

        assert_ne!(eye.pixels(), eye_off.pixels());
    }

    #[test]
    fn reuses_cached_raster_for_matching_icon_draws() {
        ICON_RASTER_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut buffer = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");
        let style = IconStyle::new(ClearColor::opaque(255, 255, 255)).with_padding(4);

        draw_icon(&mut buffer, Rect::new(0, 0, 24, 24), AssetIcon::Eye, style);
        draw_icon(&mut buffer, Rect::new(0, 0, 24, 24), AssetIcon::Eye, style);

        ICON_RASTER_CACHE.with(|cache| {
            assert_eq!(cache.borrow().len(), 1);
        });
    }
}
