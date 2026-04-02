mod blur;
mod shapes;

#[cfg(test)]
mod tests;

use tiny_skia::{FillRule, Paint, Stroke, Transform};

use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

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
        }
    }
}

pub fn draw_backdrop_layer(buffer: &mut SoftwareBuffer, rect: Rect, style: BackdropLayerStyle) {
    if rect.is_empty() {
        return;
    }

    let clipped = clip_rect(rect, buffer.size());
    if clipped.is_empty() {
        return;
    }

    match style.mode {
        BackdropLayerMode::Solid => fill_layer_shape(buffer, clipped, style),
        BackdropLayerMode::Blur => {
            blur_region(buffer, clipped, style);
            if style.color.alpha > 0 {
                fill_layer_shape(buffer, clipped, style);
            }
        }
    }

    if let Some(border_color) = style.border_color.filter(|color| color.alpha > 0)
        && style.border_width > 0
    {
        stroke_layer_shape(
            buffer,
            clipped,
            BackdropLayerStyle {
                color: border_color,
                ..style
            },
        );
    }
}

fn fill_layer_shape(buffer: &mut SoftwareBuffer, rect: Rect, style: BackdropLayerStyle) {
    if matches!(style.shape, BackdropLayerShape::Panel) && style.radius <= 0 {
        fill_rect(buffer, rect, style.color);
        return;
    }

    draw_overlay(
        buffer,
        rect.x,
        rect.y,
        rect.width.max(1) as u32,
        rect.height.max(1) as u32,
        |overlay| {
            let Some(path) = layer_path(rect.width, rect.height, style) else {
                return;
            };

            let mut paint = Paint::default();
            paint.set_color(skia_color(style.color));
            paint.anti_alias = true;
            overlay.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        },
    );
}

fn stroke_layer_shape(buffer: &mut SoftwareBuffer, rect: Rect, style: BackdropLayerStyle) {
    draw_overlay(
        buffer,
        rect.x,
        rect.y,
        rect.width.max(1) as u32,
        rect.height.max(1) as u32,
        |overlay| {
            let Some(path) = layer_path(rect.width, rect.height, style) else {
                return;
            };

            let mut paint = Paint::default();
            paint.set_color(skia_color(style.color));
            paint.anti_alias = true;

            let stroke = Stroke {
                width: style.border_width.max(1) as f32,
                ..Stroke::default()
            };
            overlay.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        },
    );
}

fn clip_rect(rect: Rect, size: FrameSize) -> Rect {
    let left = rect.x.clamp(0, size.width as i32);
    let top = rect.y.clamp(0, size.height as i32);
    let right = (rect.x + rect.width).clamp(0, size.width as i32);
    let bottom = (rect.y + rect.height).clamp(0, size.height as i32);

    Rect::new(left, top, right - left, bottom - top)
}
