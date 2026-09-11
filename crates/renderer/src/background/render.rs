use image::{Rgba, RgbaImage, imageops::FilterType};

use super::{
    BackgroundGradient, BackgroundLayered, BackgroundLayeredBase, BackgroundLayeredBlob,
    BackgroundRadial, BackgroundScaling, BackgroundTreatment, GeneratedBackground,
};
use crate::{ClearColor, FrameSize, Result, SoftwareBuffer, blur::blur_rgba};

pub(super) fn render_image(
    image: &RgbaImage,
    size: FrameSize,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
) -> Result<SoftwareBuffer> {
    let composed = match treatment.scaling {
        BackgroundScaling::Fill => render_image_fill(image, size),
        BackgroundScaling::Fit => render_image_fit(image, size, fallback),
        BackgroundScaling::Center => render_image_center(image, size, fallback),
        BackgroundScaling::Tile => render_image_tile(image, size, fallback),
        BackgroundScaling::Stretch => render_image_stretch(image, size),
    };
    let composed = blur_rgba(&composed, treatment.blur_radius, 12);
    let mut buffer = SoftwareBuffer::new(size)?;

    for (target, pixel) in buffer
        .pixels_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(composed.pixels())
    {
        target.copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Ok(buffer)
}

fn render_image_fill(image: &RgbaImage, size: FrameSize) -> RgbaImage {
    let (scaled_width, scaled_height) = cover_dimensions(
        image.width(),
        image.height(),
        size.width.max(1),
        size.height.max(1),
    );
    let resized = image::imageops::resize(image, scaled_width, scaled_height, FilterType::Triangle);
    let crop_x = (scaled_width.saturating_sub(size.width)) / 2;
    let crop_y = (scaled_height.saturating_sub(size.height)) / 2;
    image::imageops::crop_imm(&resized, crop_x, crop_y, size.width, size.height).to_image()
}

fn render_image_fit(image: &RgbaImage, size: FrameSize, fallback: ClearColor) -> RgbaImage {
    let (scaled_width, scaled_height) = fit_dimensions(
        image.width(),
        image.height(),
        size.width.max(1),
        size.height.max(1),
    );
    let resized = image::imageops::resize(image, scaled_width, scaled_height, FilterType::Triangle);
    let mut canvas = filled_canvas(size, fallback);
    blit_centered(&mut canvas, &resized);
    canvas
}

fn render_image_center(image: &RgbaImage, size: FrameSize, fallback: ClearColor) -> RgbaImage {
    let mut canvas = filled_canvas(size, fallback);
    blit_centered(&mut canvas, image);
    canvas
}

fn render_image_tile(image: &RgbaImage, size: FrameSize, fallback: ClearColor) -> RgbaImage {
    let mut canvas = filled_canvas(size, fallback);
    let tile_width = image.width().max(1);
    let tile_height = image.height().max(1);

    let mut dst_y = 0;
    while dst_y < size.height {
        let mut dst_x = 0;
        while dst_x < size.width {
            let copy_width = tile_width.min(size.width - dst_x);
            let copy_height = tile_height.min(size.height - dst_y);
            blit_region(
                &mut canvas,
                image,
                (0, 0),
                (dst_x, dst_y),
                (copy_width, copy_height),
            );
            dst_x += tile_width;
        }
        dst_y += tile_height;
    }

    canvas
}

fn render_image_stretch(image: &RgbaImage, size: FrameSize) -> RgbaImage {
    image::imageops::resize(
        image,
        size.width.max(1),
        size.height.max(1),
        FilterType::Triangle,
    )
}

pub(super) fn render_generated(
    size: FrameSize,
    generated: GeneratedBackground,
) -> Result<SoftwareBuffer> {
    match generated {
        GeneratedBackground::Gradient(gradient) => render_gradient(size, gradient),
        GeneratedBackground::Layered(layered) => render_layered(size, layered),
        GeneratedBackground::Radial(radial) => render_radial(size, radial),
    }
}

fn render_gradient(size: FrameSize, gradient: BackgroundGradient) -> Result<SoftwareBuffer> {
    let mut buffer = SoftwareBuffer::new(size)?;

    let width_span = size.width.saturating_sub(1).max(1);
    let height_span = size.height.saturating_sub(1).max(1);

    for y in 0..size.height {
        let ty = y as f32 / height_span as f32;
        for x in 0..size.width {
            let tx = x as f32 / width_span as f32;
            let color = bilerp_color(
                gradient.top_left,
                gradient.top_right,
                gradient.bottom_left,
                gradient.bottom_right,
                tx,
                ty,
            );
            let offset = ((y * size.width + x) * 4) as usize;
            buffer.pixels_mut()[offset..offset + 4].copy_from_slice(&color.to_argb8888_bytes());
        }
    }

    Ok(buffer)
}

fn render_radial(size: FrameSize, radial: BackgroundRadial) -> Result<SoftwareBuffer> {
    let mut buffer = SoftwareBuffer::new(size)?;
    let width_span = size.width.saturating_sub(1).max(1) as f32;
    let height_span = size.height.saturating_sub(1).max(1) as f32;
    let center_x = radial.center_x.min(100) as f32 / 100.0;
    let center_y = radial.center_y.min(100) as f32 / 100.0;
    let radius_scale = radial.radius.clamp(1, 200) as f32 / 100.0;
    let max_distance = max_corner_distance(center_x, center_y);
    let radius = (max_distance * radius_scale).max(f32::EPSILON);

    for y in 0..size.height {
        let py = y as f32 / height_span;
        for x in 0..size.width {
            let px = x as f32 / width_span;
            let distance = ((px - center_x).powi(2) + (py - center_y).powi(2)).sqrt();
            let t = smoothstep((distance / radius).clamp(0.0, 1.0));
            let color = lerp_color(radial.center, radial.edge, t);
            let offset = ((y * size.width + x) * 4) as usize;
            buffer.pixels_mut()[offset..offset + 4].copy_from_slice(&color.to_argb8888_bytes());
        }
    }

    Ok(buffer)
}

fn render_layered(size: FrameSize, layered: BackgroundLayered) -> Result<SoftwareBuffer> {
    let mut buffer = match layered.base {
        BackgroundLayeredBase::Solid(color) => SoftwareBuffer::solid(size, color)?,
        BackgroundLayeredBase::Gradient(gradient) => render_gradient(size, gradient)?,
        BackgroundLayeredBase::Radial(radial) => render_radial(size, radial)?,
    };

    for blob in layered.blobs.into_iter().flatten() {
        apply_layered_blob(&mut buffer, blob);
    }

    Ok(buffer)
}

fn apply_layered_blob(buffer: &mut SoftwareBuffer, blob: BackgroundLayeredBlob) {
    let size = buffer.size();
    let width_span = size.width.saturating_sub(1).max(1) as f32;
    let height_span = size.height.saturating_sub(1).max(1) as f32;
    let center_x = blob.x.min(100) as f32 / 100.0;
    let center_y = blob.y.min(100) as f32 / 100.0;
    let radius = (blob.size.clamp(1, 100) as f32 / 100.0).max(f32::EPSILON);

    for y in 0..size.height {
        let py = y as f32 / height_span;
        for x in 0..size.width {
            let px = x as f32 / width_span;
            let distance = ((px - center_x).powi(2) + (py - center_y).powi(2)).sqrt();
            let t = (distance / radius).clamp(0.0, 1.0);
            let alpha = 1.0 - smoothstep(t);
            if alpha <= 0.0 {
                continue;
            }

            let src = blob
                .color
                .with_alpha(((f32::from(blob.color.alpha) * alpha).round() as u16).min(255) as u8)
                .to_argb8888_bytes();
            let offset = ((y * size.width + x) * 4) as usize;
            blend_argb8888_pixel(&mut buffer.pixels_mut()[offset..offset + 4], &src);
        }
    }
}

fn bilerp_color(
    top_left: ClearColor,
    top_right: ClearColor,
    bottom_left: ClearColor,
    bottom_right: ClearColor,
    tx: f32,
    ty: f32,
) -> ClearColor {
    let top = lerp_color(top_left, top_right, tx);
    let bottom = lerp_color(bottom_left, bottom_right, tx);
    lerp_color(top, bottom, ty)
}

fn lerp_color(start: ClearColor, end: ClearColor, t: f32) -> ClearColor {
    ClearColor::rgba(
        lerp_channel(start.red, end.red, t),
        lerp_channel(start.green, end.green, t),
        lerp_channel(start.blue, end.blue, t),
        lerp_channel(start.alpha, end.alpha, t),
    )
}

fn lerp_channel(start: u8, end: u8, t: f32) -> u8 {
    let start = start as f32;
    let end = end as f32;
    (start + (end - start) * t).round().clamp(0.0, 255.0) as u8
}

fn max_corner_distance(center_x: f32, center_y: f32) -> f32 {
    [(0.0f32, 0.0f32), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
        .into_iter()
        .map(|(x, y)| ((x - center_x).powi(2) + (y - center_y).powi(2)).sqrt())
        .fold(0.0, f32::max)
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn blend_argb8888_pixel(dst: &mut [u8], src: &[u8; 4]) {
    let alpha = u16::from(src[3]);
    if alpha == 0 {
        return;
    }

    let inverse_alpha = 255 - alpha;
    dst[0] = blend_component(dst[0], src[0], inverse_alpha);
    dst[1] = blend_component(dst[1], src[1], inverse_alpha);
    dst[2] = blend_component(dst[2], src[2], inverse_alpha);
    dst[3] = blend_component(dst[3], src[3], inverse_alpha);
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}

fn filled_canvas(size: FrameSize, color: ClearColor) -> RgbaImage {
    RgbaImage::from_pixel(size.width, size.height, rgba_from_clear_color(color))
}

fn rgba_from_clear_color(color: ClearColor) -> Rgba<u8> {
    Rgba([color.red, color.green, color.blue, color.alpha])
}

pub(super) fn cover_dimensions(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let width_limited_height =
        (u128::from(source_height) * u128::from(target_width)).div_ceil(u128::from(source_width));
    if width_limited_height >= u128::from(target_height) {
        return (target_width, width_limited_height as u32);
    }

    let height_limited_width =
        (u128::from(source_width) * u128::from(target_height)).div_ceil(u128::from(source_height));
    (height_limited_width as u32, target_height)
}

pub(super) fn fit_dimensions(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let width_limited_height =
        (u128::from(source_height) * u128::from(target_width)) / u128::from(source_width);
    if width_limited_height <= u128::from(target_height) {
        return (target_width, width_limited_height.max(1) as u32);
    }

    let height_limited_width =
        (u128::from(source_width) * u128::from(target_height)) / u128::from(source_height);
    (height_limited_width.max(1) as u32, target_height)
}

fn blit_centered(canvas: &mut RgbaImage, image: &RgbaImage) {
    let (src_x, dst_x, width) = centered_axis(image.width(), canvas.width());
    let (src_y, dst_y, height) = centered_axis(image.height(), canvas.height());
    blit_region(
        canvas,
        image,
        (src_x, src_y),
        (dst_x, dst_y),
        (width, height),
    );
}

fn centered_axis(source: u32, target: u32) -> (u32, u32, u32) {
    if source <= target {
        let dst = (target - source) / 2;
        return (0, dst, source);
    }

    let src = (source - target) / 2;
    (src, 0, target)
}

fn blit_region(
    canvas: &mut RgbaImage,
    image: &RgbaImage,
    src_origin: (u32, u32),
    dst_origin: (u32, u32),
    size: (u32, u32),
) {
    let (src_x, src_y) = src_origin;
    let (dst_x, dst_y) = dst_origin;
    let (width, height) = size;
    if width == 0 || height == 0 {
        return;
    }

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(src_x + x, src_y + y);
            canvas.put_pixel(dst_x + x, dst_y + y, *pixel);
        }
    }
}
