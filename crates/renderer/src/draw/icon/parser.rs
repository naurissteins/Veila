use std::sync::OnceLock;

use tiny_skia::{Path, PathBuilder};

#[derive(Debug)]
pub(in crate::draw) struct ParsedIcon {
    pub(in crate::draw) path: Path,
    pub(in crate::draw) viewbox: ViewBox,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::draw) struct ViewBox {
    pub(in crate::draw) width: f32,
    pub(in crate::draw) height: f32,
}

pub(super) fn eye_icon() -> &'static ParsedIcon {
    static ICON: OnceLock<ParsedIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        parse_svg_asset(include_str!(
            "../../../../../assets/icons/eye-solid-full.svg"
        ))
    })
}

pub(super) fn eye_off_icon() -> &'static ParsedIcon {
    static ICON: OnceLock<ParsedIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        parse_svg_asset(include_str!(
            "../../../../../assets/icons/eye-slash-solid-full.svg"
        ))
    })
}

pub(super) fn user_icon() -> &'static ParsedIcon {
    static ICON: OnceLock<ParsedIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        parse_svg_asset(include_str!(
            "../../../../../assets/icons/user-regular-full.svg"
        ))
    })
}

pub(super) fn extract_path_data(svg: &str) -> Option<&str> {
    let path_start = svg.find("<path")?;
    let data_start = svg[path_start..].find("d=\"")? + path_start + 3;
    let data_end = svg[data_start..].find('"')? + data_start;
    Some(&svg[data_start..data_end])
}

pub(super) fn extract_viewbox(svg: &str) -> Option<ViewBox> {
    let viewbox_start = svg.find("viewBox=\"")? + 9;
    let viewbox_end = svg[viewbox_start..].find('"')? + viewbox_start;
    let mut parts = svg[viewbox_start..viewbox_end].split_ascii_whitespace();
    let _min_x: f32 = parts.next()?.parse().ok()?;
    let _min_y: f32 = parts.next()?.parse().ok()?;
    let width: f32 = parts.next()?.parse().ok()?;
    let height: f32 = parts.next()?.parse().ok()?;

    Some(ViewBox { width, height })
}

pub(super) fn parse_path_data(data: &str) -> Option<Path> {
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
