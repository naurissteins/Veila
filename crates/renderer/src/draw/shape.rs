use tiny_skia::{FillRule, Paint, PathBuilder, Stroke, Transform};

use crate::{ClearColor, ShadowStyle, SoftwareBuffer};

use super::skia::{color as skia_color, draw_overlay};

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

    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
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

/// Styling for a pill-shaped surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PillStyle {
    pub fill: ClearColor,
    pub border: Option<BorderStyle>,
    pub shadow: Option<ShadowStyle>,
    pub radius: i32,
}

impl PillStyle {
    pub const fn new(fill: ClearColor) -> Self {
        Self {
            fill,
            border: None,
            shadow: None,
            radius: i32::MAX,
        }
    }

    pub const fn with_border(self, border: BorderStyle) -> Self {
        Self {
            fill: self.fill,
            border: Some(border),
            shadow: self.shadow,
            radius: self.radius,
        }
    }

    pub const fn with_shadow(self, shadow: ShadowStyle) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            shadow: Some(shadow),
            radius: self.radius,
        }
    }

    pub const fn with_radius(self, radius: i32) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            shadow: self.shadow,
            radius,
        }
    }
}

/// Styling for a circle surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircleStyle {
    pub fill: ClearColor,
    pub border: Option<BorderStyle>,
    pub shadow: Option<ShadowStyle>,
}

impl CircleStyle {
    pub const fn new(fill: ClearColor) -> Self {
        Self {
            fill,
            border: None,
            shadow: None,
        }
    }

    pub const fn with_border(self, border: BorderStyle) -> Self {
        Self {
            fill: self.fill,
            border: Some(border),
            shadow: self.shadow,
        }
    }

    pub const fn with_shadow(self, shadow: ShadowStyle) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            shadow: Some(shadow),
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
    let alpha = pixel[3];
    let pixels = buffer.pixels_mut();

    for row in top as usize..bottom as usize {
        let row_start = row * stride;
        for column in left as usize..right as usize {
            let offset = row_start + column * 4;
            if alpha == u8::MAX {
                pixels[offset..offset + 4].copy_from_slice(&pixel);
            } else {
                blend_argb8888_pixel(&mut pixels[offset..offset + 4], &pixel);
            }
        }
    }
}

fn blend_argb8888_pixel(dst: &mut [u8], src: &[u8; 4]) {
    let src_alpha = u16::from(src[3]);
    if src_alpha == 0 {
        return;
    }

    if src_alpha == u16::from(u8::MAX) {
        dst.copy_from_slice(src);
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    dst[0] = blend_component(dst[0], src[0], inverse_alpha);
    dst[1] = blend_component(dst[1], src[1], inverse_alpha);
    dst[2] = blend_component(dst[2], src[2], inverse_alpha);
    dst[3] = blend_component(dst[3], src[3], inverse_alpha);
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
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

/// Draws a modern pill surface using tiny-skia.
pub fn draw_pill(buffer: &mut SoftwareBuffer, rect: Rect, style: PillStyle) {
    if rect.is_empty() {
        return;
    }

    let radius = resolved_corner_radius(rect, style.radius);
    let overlay_padding = style
        .shadow
        .map(|shadow| shadow.offset_x.abs().max(shadow.offset_y.abs()))
        .unwrap_or(0)
        + style
            .border
            .map(|border| border.thickness.max(0))
            .unwrap_or(0)
        + 2;
    let overlay_origin_x = rect.x - overlay_padding;
    let overlay_origin_y = rect.y - overlay_padding;
    let overlay_width = (rect.width + overlay_padding * 2).max(1) as u32;
    let overlay_height = (rect.height + overlay_padding * 2).max(1) as u32;

    draw_overlay(
        buffer,
        overlay_origin_x,
        overlay_origin_y,
        overlay_width,
        overlay_height,
        |overlay| {
            let offset_x = overlay_padding as f32;
            let offset_y = overlay_padding as f32;

            if let Some(shadow) = style.shadow {
                fill_rounded_rect_path(
                    overlay,
                    rect.width,
                    rect.height,
                    radius,
                    shadow.color,
                    offset_x + shadow.offset_x as f32,
                    offset_y + shadow.offset_y as f32,
                );
            }

            if let Some(border) = style.border {
                fill_rounded_rect_path(
                    overlay,
                    rect.width,
                    rect.height,
                    radius,
                    border.color,
                    offset_x,
                    offset_y,
                );
                let inset = border.thickness.max(1);
                let inner_width = rect.width - inset * 2;
                let inner_height = rect.height - inset * 2;
                if inner_width > 0 && inner_height > 0 {
                    fill_rounded_rect_path(
                        overlay,
                        inner_width,
                        inner_height,
                        (radius - inset).max(0),
                        style.fill,
                        offset_x + inset as f32,
                        offset_y + inset as f32,
                    );
                }
            } else {
                fill_rounded_rect_path(
                    overlay,
                    rect.width,
                    rect.height,
                    radius,
                    style.fill,
                    offset_x,
                    offset_y,
                );
            }
        },
    );
}

/// Draws a filled circle with optional border and shadow using tiny-skia.
pub fn draw_circle(
    buffer: &mut SoftwareBuffer,
    center_x: i32,
    center_y: i32,
    radius: i32,
    style: CircleStyle,
) {
    if radius <= 0 {
        return;
    }

    let diameter = radius * 2;
    let bounds = Rect::new(center_x - radius, center_y - radius, diameter, diameter);
    let overlay_padding = style
        .shadow
        .map(|shadow| shadow.offset_x.abs().max(shadow.offset_y.abs()))
        .unwrap_or(0)
        + style
            .border
            .map(|border| border.thickness.max(0))
            .unwrap_or(0)
        + 2;
    let overlay_origin_x = bounds.x - overlay_padding;
    let overlay_origin_y = bounds.y - overlay_padding;
    let overlay_size = (diameter + overlay_padding * 2).max(1) as u32;

    draw_overlay(
        buffer,
        overlay_origin_x,
        overlay_origin_y,
        overlay_size,
        overlay_size,
        |overlay| {
            let center_x = (overlay_padding + radius) as f32;
            let center_y = (overlay_padding + radius) as f32;

            if let Some(shadow) = style.shadow {
                fill_circle_path(
                    overlay,
                    center_x + shadow.offset_x as f32,
                    center_y + shadow.offset_y as f32,
                    radius as f32,
                    shadow.color,
                );
            }

            if let Some(border) = style.border {
                let border_thickness = border.thickness.max(1).min(radius);
                let inner_radius = (radius - border_thickness).max(0) as f32;
                if inner_radius > 0.0 {
                    fill_circle_path(overlay, center_x, center_y, inner_radius, style.fill);
                }
                stroke_circle_path(
                    overlay,
                    center_x,
                    center_y,
                    radius as f32 - border_thickness as f32 / 2.0,
                    border_thickness as f32,
                    border.color,
                );
            } else {
                fill_circle_path(overlay, center_x, center_y, radius as f32, style.fill);
            }
        },
    );
}

fn resolved_corner_radius(rect: Rect, radius: i32) -> i32 {
    let max_radius = (rect.width.min(rect.height) / 2).max(0);
    if radius == i32::MAX {
        max_radius
    } else {
        radius.clamp(0, max_radius)
    }
}

fn fill_rounded_rect_path(
    overlay: &mut tiny_skia::Pixmap,
    width: i32,
    height: i32,
    radius: i32,
    color: ClearColor,
    offset_x: f32,
    offset_y: f32,
) {
    if width <= 0 || height <= 0 {
        return;
    }

    let right = offset_x + width as f32;
    let bottom = offset_y + height as f32;
    let radius = radius.max(0) as f32;
    let mut builder = PathBuilder::new();

    if radius <= 0.0 {
        builder.move_to(offset_x, offset_y);
        builder.line_to(right, offset_y);
        builder.line_to(right, bottom);
        builder.line_to(offset_x, bottom);
    } else {
        builder.move_to(offset_x + radius, offset_y);
        builder.line_to(right - radius, offset_y);
        builder.quad_to(right, offset_y, right, offset_y + radius);
        builder.line_to(right, bottom - radius);
        builder.quad_to(right, bottom, right - radius, bottom);
        builder.line_to(offset_x + radius, bottom);
        builder.quad_to(offset_x, bottom, offset_x, bottom - radius);
        builder.line_to(offset_x, offset_y + radius);
        builder.quad_to(offset_x, offset_y, offset_x + radius, offset_y);
    }

    builder.close();
    let Some(path) = builder.finish() else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;
    overlay.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn fill_circle_path(
    overlay: &mut tiny_skia::Pixmap,
    center_x: f32,
    center_y: f32,
    radius: f32,
    color: ClearColor,
) {
    let Some(path) = PathBuilder::from_circle(center_x, center_y, radius.max(1.0)) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;
    overlay.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn stroke_circle_path(
    overlay: &mut tiny_skia::Pixmap,
    center_x: f32,
    center_y: f32,
    radius: f32,
    width: f32,
    color: ClearColor,
) {
    if radius <= 0.0 || width <= 0.0 {
        return;
    }

    let Some(path) = PathBuilder::from_circle(center_x, center_y, radius) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;

    let stroke = Stroke {
        width,
        ..Stroke::default()
    };
    overlay.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

#[cfg(test)]
mod tests {
    use super::{
        BorderStyle, BoxStyle, CircleStyle, PillStyle, Rect, draw_box, draw_circle, draw_pill,
        fill_rect, stroke_rect,
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
    fn blends_translucent_rectangles_over_existing_pixels() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(1, 1), ClearColor::opaque(10, 20, 30))
                .expect("buffer");
        fill_rect(
            &mut buffer,
            Rect::new(0, 0, 1, 1),
            ClearColor::rgba(255, 255, 255, 128),
        );

        assert_eq!(buffer.pixels(), &[143, 138, 133, 255]);
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

    #[test]
    fn draws_pill_surface() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 80)).expect("buffer");
        draw_pill(
            &mut buffer,
            Rect::new(16, 24, 88, 32),
            PillStyle::new(ClearColor::rgba(12, 18, 28, 210))
                .with_border(BorderStyle::new(ClearColor::opaque(255, 255, 255), 2)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn draws_pill_surface_at_large_offsets() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(960, 540)).expect("buffer");
        draw_pill(
            &mut buffer,
            Rect::new(320, 240, 280, 56),
            PillStyle::new(ClearColor::rgba(12, 18, 28, 232))
                .with_border(BorderStyle::new(ClearColor::opaque(92, 108, 146), 2)),
        );

        let row_start = (268 * 960 + 460) * 4;
        assert_ne!(&buffer.pixels()[row_start..row_start + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn draws_pill_surface_with_custom_radius() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(96, 72)).expect("buffer");
        draw_pill(
            &mut buffer,
            Rect::new(12, 16, 72, 32),
            PillStyle::new(ClearColor::rgba(12, 18, 28, 232))
                .with_border(BorderStyle::new(ClearColor::opaque(92, 108, 146), 2))
                .with_radius(10),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn blends_translucent_pill_fill_without_overdarkening() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(24, 24), ClearColor::opaque(200, 100, 0))
                .expect("buffer");
        draw_pill(
            &mut buffer,
            Rect::new(4, 4, 16, 16),
            PillStyle::new(ClearColor::rgba(255, 255, 255, 51)),
        );

        let center = (12 * 24 + 12) * 4;
        assert_eq!(&buffer.pixels()[center..center + 4], &[51, 131, 211, 255]);
    }

    #[test]
    fn draws_circle_surface() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(80, 80)).expect("buffer");
        draw_circle(
            &mut buffer,
            40,
            40,
            20,
            CircleStyle::new(ClearColor::rgba(240, 244, 250, 220))
                .with_border(BorderStyle::new(ClearColor::opaque(20, 24, 32), 2)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn keeps_translucent_circle_fill_free_from_border_tint() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(64, 64), ClearColor::opaque(200, 100, 0))
                .expect("buffer");
        draw_circle(
            &mut buffer,
            32,
            32,
            20,
            CircleStyle::new(ClearColor::rgba(255, 255, 255, 15))
                .with_border(BorderStyle::new(ClearColor::rgba(148, 178, 255, 108), 2)),
        );

        let center = (32 * 64 + 32) * 4;
        assert_eq!(&buffer.pixels()[center..center + 4], &[15, 109, 203, 255]);
    }
}
