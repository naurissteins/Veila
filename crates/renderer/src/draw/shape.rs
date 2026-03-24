use crate::{ClearColor, SoftwareBuffer};

/// Rectangle in buffer pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// Creates a rectangle.
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns whether the rectangle has drawable area.
    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }
}

/// Border configuration for a filled box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderStyle {
    pub color: ClearColor,
    pub thickness: i32,
}

impl BorderStyle {
    /// Creates a border style.
    pub const fn new(color: ClearColor, thickness: i32) -> Self {
        Self { color, thickness }
    }
}

/// Fill and optional border styling for a rectangular box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxStyle {
    pub fill: ClearColor,
    pub border: Option<BorderStyle>,
}

impl BoxStyle {
    /// Creates a filled box style.
    pub const fn new(fill: ClearColor) -> Self {
        Self { fill, border: None }
    }

    /// Adds a border to a box style.
    pub const fn with_border(self, border: BorderStyle) -> Self {
        Self {
            fill: self.fill,
            border: Some(border),
        }
    }
}

/// Fills a rectangle.
pub fn fill_rect(buffer: &mut SoftwareBuffer, rect: Rect, color: ClearColor) {
    if rect.is_empty() {
        return;
    }

    let size = buffer.size();
    let right = (rect.x + rect.width).clamp(0, size.width as i32);
    let bottom = (rect.y + rect.height).clamp(0, size.height as i32);
    let left = rect.x.clamp(0, size.width as i32);
    let top = rect.y.clamp(0, size.height as i32);

    if left >= right || top >= bottom {
        return;
    }

    let stride = size.width as usize * 4;
    let pixel = color.to_argb8888_bytes();
    let pixels = buffer.pixels_mut();

    for row in top as usize..bottom as usize {
        let row_start = row * stride;
        for column in left as usize..right as usize {
            let offset = row_start + column * 4;
            pixels[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
}

/// Draws a border around a rectangle.
pub fn stroke_rect(buffer: &mut SoftwareBuffer, rect: Rect, border: BorderStyle) {
    if rect.is_empty() || border.thickness <= 0 {
        return;
    }

    fill_rect(
        buffer,
        Rect::new(rect.x, rect.y, rect.width, border.thickness),
        border.color,
    );
    fill_rect(
        buffer,
        Rect::new(
            rect.x,
            rect.y + rect.height - border.thickness,
            rect.width,
            border.thickness,
        ),
        border.color,
    );
    fill_rect(
        buffer,
        Rect::new(rect.x, rect.y, border.thickness, rect.height),
        border.color,
    );
    fill_rect(
        buffer,
        Rect::new(
            rect.x + rect.width - border.thickness,
            rect.y,
            border.thickness,
            rect.height,
        ),
        border.color,
    );
}

/// Draws a filled box with an optional border.
pub fn draw_box(buffer: &mut SoftwareBuffer, rect: Rect, style: BoxStyle) {
    fill_rect(buffer, rect, style.fill);

    if let Some(border) = style.border {
        stroke_rect(buffer, rect, border);
    }
}

#[cfg(test)]
mod tests {
    use super::{BorderStyle, BoxStyle, Rect, draw_box, fill_rect, stroke_rect};
    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    #[test]
    fn fills_rectangles() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(8, 8)).expect("buffer");
        fill_rect(
            &mut buffer,
            Rect::new(2, 2, 3, 3),
            ClearColor::opaque(255, 255, 255),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn strokes_rectangles() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(8, 8)).expect("buffer");
        stroke_rect(
            &mut buffer,
            Rect::new(1, 1, 6, 6),
            BorderStyle::new(ClearColor::opaque(255, 255, 255), 1),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn draws_box_with_border() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(12, 12)).expect("buffer");
        draw_box(
            &mut buffer,
            Rect::new(1, 1, 10, 10),
            BoxStyle::new(ClearColor::opaque(8, 10, 14))
                .with_border(BorderStyle::new(ClearColor::opaque(255, 255, 255), 1)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
