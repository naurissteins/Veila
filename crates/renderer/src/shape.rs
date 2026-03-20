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

    /// Returns an inset rectangle.
    pub const fn inset(self, amount: i32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: self.width - amount * 2,
            height: self.height - amount * 2,
        }
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

/// Shadow configuration for a filled box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxShadow {
    pub color: ClearColor,
    pub blur_radius: i32,
    pub spread: i32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl BoxShadow {
    /// Creates a shadow style.
    pub const fn new(
        color: ClearColor,
        blur_radius: i32,
        spread: i32,
        offset_x: i32,
        offset_y: i32,
    ) -> Self {
        Self {
            color,
            blur_radius,
            spread,
            offset_x,
            offset_y,
        }
    }
}

/// Fill and optional border styling for a rectangular box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxStyle {
    pub fill: ClearColor,
    pub border: Option<BorderStyle>,
    pub radius: i32,
    pub shadow: Option<BoxShadow>,
}

impl BoxStyle {
    /// Creates a filled box style.
    pub const fn new(fill: ClearColor) -> Self {
        Self {
            fill,
            border: None,
            radius: 0,
            shadow: None,
        }
    }

    /// Adds a border to a box style.
    pub const fn with_border(self, border: BorderStyle) -> Self {
        Self {
            fill: self.fill,
            border: Some(border),
            radius: self.radius,
            shadow: self.shadow,
        }
    }

    /// Adds a uniform corner radius.
    pub const fn with_radius(self, radius: i32) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            radius,
            shadow: self.shadow,
        }
    }

    /// Adds a soft shadow.
    pub const fn with_shadow(self, shadow: BoxShadow) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            radius: self.radius,
            shadow: Some(shadow),
        }
    }
}

/// Fills a rectangle.
pub fn fill_rect(buffer: &mut SoftwareBuffer, rect: Rect, color: ClearColor) {
    fill_round_rect(buffer, rect, color, 0);
}

/// Fills a rounded rectangle.
pub fn fill_round_rect(buffer: &mut SoftwareBuffer, rect: Rect, color: ClearColor, radius: i32) {
    draw_round_rect_mask(buffer, rect, radius, color, None);
}

/// Draws a border around a rectangle.
pub fn stroke_rect(buffer: &mut SoftwareBuffer, rect: Rect, border: BorderStyle) {
    stroke_round_rect(buffer, rect, border, 0);
}

/// Draws a border around a rounded rectangle.
pub fn stroke_round_rect(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    border: BorderStyle,
    radius: i32,
) {
    if rect.is_empty() || border.thickness <= 0 {
        return;
    }

    let inner_rect = rect.inset(border.thickness);
    let inner_radius = (radius - border.thickness).max(0);
    draw_round_rect_mask(
        buffer,
        rect,
        radius,
        border.color,
        Some((inner_rect, inner_radius)),
    );
}

/// Draws a filled box with an optional border.
pub fn draw_box(buffer: &mut SoftwareBuffer, rect: Rect, style: BoxStyle) {
    if let Some(shadow) = style.shadow {
        draw_box_shadow(buffer, rect, style.radius, shadow);
    }

    fill_round_rect(buffer, rect, style.fill, style.radius);

    if let Some(border) = style.border {
        stroke_round_rect(buffer, rect, border, style.radius);
    }
}

fn draw_box_shadow(buffer: &mut SoftwareBuffer, rect: Rect, radius: i32, shadow: BoxShadow) {
    if rect.is_empty() || shadow.color.alpha == 0 {
        return;
    }

    let shadow_rect = Rect::new(
        rect.x + shadow.offset_x - shadow.spread,
        rect.y + shadow.offset_y - shadow.spread,
        rect.width + shadow.spread * 2,
        rect.height + shadow.spread * 2,
    );
    let blur = shadow.blur_radius.max(0) as f32;
    let left = shadow_rect.x - shadow.blur_radius;
    let top = shadow_rect.y - shadow.blur_radius;
    let right = shadow_rect.x + shadow_rect.width + shadow.blur_radius;
    let bottom = shadow_rect.y + shadow_rect.height + shadow.blur_radius;
    let size = buffer.size();

    for py in top.max(0)..bottom.min(size.height as i32) {
        for px in left.max(0)..right.min(size.width as i32) {
            let distance = rounded_rect_distance(
                px as f32 + 0.5,
                py as f32 + 0.5,
                shadow_rect,
                radius + shadow.spread,
            );
            let coverage = if blur <= 0.0 {
                hard_coverage(distance)
            } else {
                ((blur + 0.5 - distance) / (blur + 0.5)).clamp(0.0, 1.0)
            };
            if coverage <= 0.0 {
                continue;
            }

            let alpha = (shadow.color.alpha as f32 * coverage * coverage) as u8;
            if alpha == 0 {
                continue;
            }

            blend_pixel(
                buffer,
                px,
                py,
                ClearColor::rgba(
                    shadow.color.red,
                    shadow.color.green,
                    shadow.color.blue,
                    alpha,
                ),
            );
        }
    }
}

fn draw_round_rect_mask(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    radius: i32,
    color: ClearColor,
    punch_out: Option<(Rect, i32)>,
) {
    if rect.is_empty() || color.alpha == 0 {
        return;
    }

    let size = buffer.size();
    let left = rect.x.clamp(0, size.width as i32);
    let top = rect.y.clamp(0, size.height as i32);
    let right = (rect.x + rect.width).clamp(0, size.width as i32);
    let bottom = (rect.y + rect.height).clamp(0, size.height as i32);

    if left >= right || top >= bottom {
        return;
    }

    for py in top..bottom {
        for px in left..right {
            let outer = coverage_for_rect(px, py, rect, radius);
            if outer <= 0.0 {
                continue;
            }

            let coverage = if let Some((inner_rect, inner_radius)) = punch_out {
                outer * (1.0 - coverage_for_rect(px, py, inner_rect, inner_radius))
            } else {
                outer
            };

            if coverage <= 0.0 {
                continue;
            }

            let alpha = (color.alpha as f32 * coverage) as u8;
            if alpha == 0 {
                continue;
            }

            blend_pixel(
                buffer,
                px,
                py,
                ClearColor::rgba(color.red, color.green, color.blue, alpha),
            );
        }
    }
}

fn coverage_for_rect(x: i32, y: i32, rect: Rect, radius: i32) -> f32 {
    let distance = rounded_rect_distance(x as f32 + 0.5, y as f32 + 0.5, rect, radius);
    smooth_coverage(distance)
}

fn rounded_rect_distance(px: f32, py: f32, rect: Rect, radius: i32) -> f32 {
    let half_width = rect.width.max(1) as f32 / 2.0;
    let half_height = rect.height.max(1) as f32 / 2.0;
    let radius = radius.max(0) as f32;
    let max_radius = half_width.min(half_height);
    let radius = radius.min(max_radius);
    let inner_x = (half_width - radius).max(0.0);
    let inner_y = (half_height - radius).max(0.0);
    let center_x = rect.x as f32 + half_width;
    let center_y = rect.y as f32 + half_height;
    let qx = (px - center_x).abs() - inner_x;
    let qy = (py - center_y).abs() - inner_y;
    let outside_x = qx.max(0.0);
    let outside_y = qy.max(0.0);
    let outside = (outside_x * outside_x + outside_y * outside_y).sqrt();
    let inside = qx.max(qy).min(0.0);

    outside + inside - radius
}

fn smooth_coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

fn hard_coverage(distance: f32) -> f32 {
    if distance <= 0.0 { 1.0 } else { 0.0 }
}

fn blend_pixel(buffer: &mut SoftwareBuffer, x: i32, y: i32, color: ClearColor) {
    let size = buffer.size();
    if x < 0 || y < 0 || x >= size.width as i32 || y >= size.height as i32 || color.alpha == 0 {
        return;
    }

    let stride = size.width as usize * 4;
    let offset = y as usize * stride + x as usize * 4;
    let pixels = buffer.pixels_mut();
    let src_alpha = color.alpha as u16;

    if src_alpha == u8::MAX as u16 {
        let pixel = color.to_argb8888_bytes();
        pixels[offset..offset + 4].copy_from_slice(&pixel);
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    let dst_alpha = pixels[offset + 3] as u16;

    pixels[offset] = blend_channel(color.blue, pixels[offset], src_alpha, inverse_alpha);
    pixels[offset + 1] = blend_channel(color.green, pixels[offset + 1], src_alpha, inverse_alpha);
    pixels[offset + 2] = blend_channel(color.red, pixels[offset + 2], src_alpha, inverse_alpha);
    pixels[offset + 3] = (src_alpha + (dst_alpha * inverse_alpha) / u16::from(u8::MAX))
        .min(u16::from(u8::MAX)) as u8;
}

fn blend_channel(src: u8, dst: u8, src_alpha: u16, inverse_alpha: u16) -> u8 {
    (((src as u16 * src_alpha) + (dst as u16 * inverse_alpha)) / u16::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::{
        BorderStyle, BoxShadow, BoxStyle, Rect, draw_box, fill_rect, fill_round_rect, stroke_rect,
    };
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
    fn fills_rounded_rectangles() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(16, 16)).expect("buffer");
        fill_round_rect(
            &mut buffer,
            Rect::new(2, 2, 12, 12),
            ClearColor::opaque(255, 255, 255),
            5,
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
                .with_radius(4)
                .with_border(BorderStyle::new(ClearColor::opaque(255, 255, 255), 1)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn draws_box_with_shadow() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(32, 32)).expect("buffer");
        draw_box(
            &mut buffer,
            Rect::new(8, 8, 12, 12),
            BoxStyle::new(ClearColor::opaque(8, 10, 14))
                .with_radius(6)
                .with_shadow(BoxShadow::new(ClearColor::rgba(0, 0, 0, 120), 8, 0, 0, 4)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
