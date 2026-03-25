use std::sync::OnceLock;

use tiny_skia::{FillRule, Paint, Path, PathBuilder, Transform};

use crate::{ClearColor, SoftwareBuffer, shape::Rect};

use super::skia::{color as skia_color, draw_overlay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetIcon {
    Eye,
    EyeOff,
    User,
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
    let parsed = match icon {
        AssetIcon::Eye => eye_icon(),
        AssetIcon::EyeOff => eye_off_icon(),
        AssetIcon::User => user_icon(),
    };

    draw_overlay(buffer, rect.x, rect.y, width, height, |pixmap| {
        let inset = style.padding.max(0) as f32;
        let target_width = (width as f32 - inset * 2.0).max(1.0);
        let target_height = (height as f32 - inset * 2.0).max(1.0);
        let scale =
            (target_width / parsed.viewbox.width).min(target_height / parsed.viewbox.height);
        let icon_width = parsed.viewbox.width * scale;
        let icon_height = parsed.viewbox.height * scale;
        let translate_x = ((width as f32 - icon_width) / 2.0).max(0.0);
        let translate_y = ((height as f32 - icon_height) / 2.0).max(0.0);
        let transform =
            Transform::from_scale(scale, scale).post_translate(translate_x, translate_y);
        let mut paint = Paint::default();
        paint.set_color(skia_color(style.color));
        paint.anti_alias = true;
        pixmap.fill_path(&parsed.path, &paint, FillRule::Winding, transform, None);
    });
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
        AssetIcon, IconStyle, draw_icon, extract_path_data, extract_viewbox, parse_path_data,
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
}
