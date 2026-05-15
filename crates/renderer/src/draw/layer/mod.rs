mod blur;
mod shapes;

#[cfg(test)]
mod tests;

use tiny_skia::{FillRule, Paint, Stroke, Transform};

use crate::{ClearColor, FrameSize, PixelBuffer, shape::Rect};

use super::{
    shape::fill_rect,
    skia::{color as skia_color, draw_overlay},
};

use blur::blur_region;
use shapes::layer_path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BackdropLayerAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BackdropLayerShape {
    #[default]
    Panel,
    Diagonal(BackdropLayerAlignment),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BackdropLayerMode {
    Solid,
    #[default]
    Blur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackdropLayerStyle {
    pub mode: BackdropLayerMode,
    pub shape: BackdropLayerShape,
    pub color: ClearColor,
    pub blur_radius: u8,
    pub radius: i32,
    pub border_color: Option<ClearColor>,
    pub border_width: i32,
    pub rotate_degrees: i16,
}

impl BackdropLayerStyle {
    pub const fn new(
        mode: BackdropLayerMode,
        shape: BackdropLayerShape,
        color: ClearColor,
        blur_radius: u8,
        radius: i32,
        border_color: Option<ClearColor>,
        border_width: i32,
    ) -> Self {
        Self {
            mode,
            shape,
            color,
            blur_radius,
            radius,
            border_color,
            border_width,
            rotate_degrees: 0,
        }
    }

    pub const fn with_rotation(mut self, rotate_degrees: i16) -> Self {
        self.rotate_degrees = rotate_degrees;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LayerSurface {
    pub rect: Rect,
    pub bounds: Rect,
    pub rotate_degrees: i16,
}

impl LayerSurface {
    fn new(rect: Rect, size: FrameSize, rotate_degrees: i16) -> Option<Self> {
        let rotate_degrees = rotate_degrees.rem_euclid(360);
        let bounds = if rotate_degrees == 0 {
            rect
        } else {
            rotated_bounds(rect, rotate_degrees)
        };
        let bounds = clip_rect(bounds, size);
        (!bounds.is_empty()).then_some(Self {
            rect,
            bounds,
            rotate_degrees,
        })
    }

    pub(super) fn transform(self) -> Transform {
        let origin_x = (self.rect.x - self.bounds.x) as f32;
        let origin_y = (self.rect.y - self.bounds.y) as f32;
        if self.rotate_degrees == 0 {
            return Transform::from_translate(origin_x, origin_y);
        }

        let half_width = self.rect.width as f32 / 2.0;
        let half_height = self.rect.height as f32 / 2.0;
        let center_x = origin_x + half_width;
        let center_y = origin_y + half_height;
        let radians = (self.rotate_degrees as f32).to_radians();
        let sin = radians.sin();
        let cos = radians.cos();

        Transform::from_row(
            cos,
            sin,
            -sin,
            cos,
            center_x - cos * half_width + sin * half_height,
            center_y - sin * half_width - cos * half_height,
        )
    }
}

pub fn draw_backdrop_layer(buffer: &mut impl PixelBuffer, rect: Rect, style: BackdropLayerStyle) {
    if rect.is_empty() {
        return;
    }

    let Some(surface) = LayerSurface::new(rect, buffer.size(), style.rotate_degrees) else {
        return;
    };

    match style.mode {
        BackdropLayerMode::Solid => fill_layer_shape(buffer, surface, style),
        BackdropLayerMode::Blur => {
            blur_region(buffer, surface, style);
            if style.color.alpha > 0 {
                fill_layer_shape(buffer, surface, style);
            }
        }
    }

    if let Some(border_color) = style.border_color.filter(|color| color.alpha > 0)
        && style.border_width > 0
    {
        stroke_layer_shape(
            buffer,
            surface,
            BackdropLayerStyle {
                color: border_color,
                ..style
            },
        );
    }
}

fn fill_layer_shape(
    buffer: &mut impl PixelBuffer,
    surface: LayerSurface,
    style: BackdropLayerStyle,
) {
    if surface.rotate_degrees == 0
        && matches!(style.shape, BackdropLayerShape::Panel)
        && style.radius <= 0
    {
        fill_rect(buffer, surface.bounds, style.color);
        return;
    }

    draw_overlay(
        buffer,
        surface.bounds.x,
        surface.bounds.y,
        surface.bounds.width.max(1) as u32,
        surface.bounds.height.max(1) as u32,
        |overlay| {
            let Some(path) = layer_path(surface.rect.width, surface.rect.height, style) else {
                return;
            };

            let mut paint = Paint::default();
            paint.set_color(skia_color(style.color));
            paint.anti_alias = true;
            overlay.fill_path(&path, &paint, FillRule::Winding, surface.transform(), None);
        },
    );
}

fn stroke_layer_shape(
    buffer: &mut impl PixelBuffer,
    surface: LayerSurface,
    style: BackdropLayerStyle,
) {
    draw_overlay(
        buffer,
        surface.bounds.x,
        surface.bounds.y,
        surface.bounds.width.max(1) as u32,
        surface.bounds.height.max(1) as u32,
        |overlay| {
            let Some(path) = layer_path(surface.rect.width, surface.rect.height, style) else {
                return;
            };

            let mut paint = Paint::default();
            paint.set_color(skia_color(style.color));
            paint.anti_alias = true;

            let stroke = Stroke {
                width: style.border_width.max(1) as f32,
                ..Stroke::default()
            };
            overlay.stroke_path(&path, &paint, &stroke, surface.transform(), None);
        },
    );
}

fn rotated_bounds(rect: Rect, rotate_degrees: i16) -> Rect {
    let half_width = rect.width as f32 / 2.0;
    let half_height = rect.height as f32 / 2.0;
    let center_x = rect.x as f32 + half_width;
    let center_y = rect.y as f32 + half_height;
    let radians = (rotate_degrees as f32).to_radians();
    let sin = radians.sin();
    let cos = radians.cos();
    let corners = [
        (-half_width, -half_height),
        (half_width, -half_height),
        (half_width, half_height),
        (-half_width, half_height),
    ];

    let mut left = f32::INFINITY;
    let mut top = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::NEG_INFINITY;

    for (x, y) in corners {
        let rotated_x = center_x + x * cos - y * sin;
        let rotated_y = center_y + x * sin + y * cos;
        left = left.min(rotated_x);
        top = top.min(rotated_y);
        right = right.max(rotated_x);
        bottom = bottom.max(rotated_y);
    }

    let x = left.floor() as i32;
    let y = top.floor() as i32;
    Rect::new(
        x,
        y,
        (right.ceil() as i32 - x).max(1),
        (bottom.ceil() as i32 - y).max(1),
    )
}

fn clip_rect(rect: Rect, size: FrameSize) -> Rect {
    let left = rect.x.clamp(0, size.width as i32);
    let top = rect.y.clamp(0, size.height as i32);
    let right = (rect.x + rect.width).clamp(0, size.width as i32);
    let bottom = (rect.y + rect.height).clamp(0, size.height as i32);

    Rect::new(left, top, right - left, bottom - top)
}
