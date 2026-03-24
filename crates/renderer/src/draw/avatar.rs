use std::path::Path;

use image::RgbaImage;
use tiny_skia::{
    FillRule, FilterQuality, LineCap, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke,
    Transform,
};

use crate::{ClearColor, FrameSize, RendererError, Result, ShadowStyle, SoftwareBuffer};

use super::{
    shape::{BorderStyle, CircleStyle, draw_circle},
    skia::{color as skia_color, draw_overlay},
};

#[derive(Debug, Clone)]
pub enum AvatarAsset {
    Image(Pixmap),
    Placeholder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvatarStyle {
    pub background: ClearColor,
    pub placeholder: ClearColor,
    pub ring: Option<BorderStyle>,
    pub shadow: Option<ShadowStyle>,
}

impl AvatarStyle {
    pub const fn new(background: ClearColor, placeholder: ClearColor) -> Self {
        Self {
            background,
            placeholder,
            ring: None,
            shadow: None,
        }
    }

    pub const fn with_ring(self, ring: BorderStyle) -> Self {
        Self {
            background: self.background,
            placeholder: self.placeholder,
            ring: Some(ring),
            shadow: self.shadow,
        }
    }

    pub const fn with_shadow(self, shadow: ShadowStyle) -> Self {
        Self {
            background: self.background,
            placeholder: self.placeholder,
            ring: self.ring,
            shadow: Some(shadow),
        }
    }
}

impl AvatarAsset {
    pub fn load(path: &Path) -> Result<Self> {
        let image = image::open(path)?.to_rgba8();
        let pixmap = rgba_to_pixmap(image)?;
        Ok(Self::Image(pixmap))
    }

    pub const fn placeholder() -> Self {
        Self::Placeholder
    }

    pub fn draw(
        &self,
        buffer: &mut SoftwareBuffer,
        center_x: i32,
        top_y: i32,
        size: u32,
        style: AvatarStyle,
    ) {
        if size == 0 {
            return;
        }

        let radius = (size as i32 / 2).max(1);
        let center_y = top_y + radius;
        let mut circle_style = CircleStyle::new(style.background);
        if let Some(shadow) = style.shadow {
            circle_style = circle_style.with_shadow(shadow);
        }
        if let Some(ring) = style.ring {
            circle_style = circle_style.with_border(ring);
        }
        draw_circle(buffer, center_x, center_y, radius, circle_style);

        let inset = style
            .ring
            .map(|ring| ring.thickness.max(0) * 2)
            .unwrap_or(0);
        let content_size = (size as i32 - inset * 2).max(1) as u32;
        let content_top = top_y + inset;
        let content_left = center_x - content_size as i32 / 2;

        match self {
            Self::Image(image) => {
                draw_avatar_image(buffer, content_left, content_top, content_size, image)
            }
            Self::Placeholder => draw_placeholder(
                buffer,
                center_x,
                content_top,
                content_size,
                style.placeholder,
            ),
        }
    }
}

fn draw_avatar_image(buffer: &mut SoftwareBuffer, left: i32, top: i32, size: u32, image: &Pixmap) {
    draw_overlay(buffer, left, top, size, size, |overlay| {
        let Some(mut mask) = Mask::new(size, size) else {
            return;
        };
        let Some(circle) =
            PathBuilder::from_circle(size as f32 / 2.0, size as f32 / 2.0, size as f32 / 2.0)
        else {
            return;
        };
        mask.fill_path(&circle, FillRule::Winding, true, Transform::identity());

        let paint = PixmapPaint {
            quality: FilterQuality::Bicubic,
            ..PixmapPaint::default()
        };
        let scale = f32::max(
            size as f32 / image.width() as f32,
            size as f32 / image.height() as f32,
        );
        let translate_x = (size as f32 - image.width() as f32 * scale) / 2.0;
        let translate_y = (size as f32 - image.height() as f32 * scale) / 2.0;
        let transform = Transform::from_row(scale, 0.0, 0.0, scale, translate_x, translate_y);

        overlay.draw_pixmap(0, 0, image.as_ref(), &paint, transform, Some(&mask));
    });
}

fn draw_placeholder(
    buffer: &mut SoftwareBuffer,
    center_x: i32,
    top_y: i32,
    size: u32,
    color: ClearColor,
) {
    let radius = size as i32 / 2;
    let overlay_left = center_x - radius;
    draw_overlay(buffer, overlay_left, top_y, size, size, |overlay| {
        let center = size as f32 / 2.0;
        let head_radius = size as f32 * 0.18;
        fill_circle(overlay, center, size as f32 * 0.36, head_radius, color);

        let mut builder = PathBuilder::new();
        let shoulders_y = size as f32 * 0.74;
        builder.move_to(size as f32 * 0.28, shoulders_y);
        builder.line_to(size as f32 * 0.72, shoulders_y);
        let Some(path) = builder.finish() else {
            return;
        };

        let mut paint = Paint::default();
        paint.set_color(skia_color(color));
        paint.anti_alias = true;

        let stroke = Stroke {
            width: size as f32 * 0.22,
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        overlay.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    });
}

fn fill_circle(overlay: &mut Pixmap, center_x: f32, center_y: f32, radius: f32, color: ClearColor) {
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

fn rgba_to_pixmap(image: RgbaImage) -> Result<Pixmap> {
    let width = image.width();
    let height = image.height();
    let size = tiny_skia::IntSize::from_wh(width, height).ok_or(
        RendererError::InvalidFrameSize(FrameSize::new(width, height)),
    )?;
    let mut data = image.into_raw();
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3];
        pixel[0] = premultiply(pixel[0], alpha);
        pixel[1] = premultiply(pixel[1], alpha);
        pixel[2] = premultiply(pixel[2], alpha);
    }
    Pixmap::from_vec(data, size).ok_or(RendererError::InvalidFrameSize(FrameSize::new(
        width, height,
    )))
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{AvatarAsset, AvatarStyle, rgba_to_pixmap};
    use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::BorderStyle};

    #[test]
    fn converts_rgba_image_to_pixmap() {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([120, 80, 40, 255]));
        let pixmap = rgba_to_pixmap(image).expect("pixmap");

        assert_eq!(pixmap.data(), &[120, 80, 40, 255]);
    }

    #[test]
    fn draws_placeholder_avatar() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(160, 160)).expect("buffer");
        AvatarAsset::placeholder().draw(
            &mut buffer,
            80,
            20,
            96,
            AvatarStyle::new(
                ClearColor::rgba(255, 255, 255, 36),
                ClearColor::opaque(240, 244, 250),
            )
            .with_ring(BorderStyle::new(ClearColor::rgba(255, 255, 255, 72), 2)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
