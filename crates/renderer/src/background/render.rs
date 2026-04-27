use image::{RgbaImage, imageops::FilterType};

use super::{BackgroundGradient, BackgroundTreatment};
use crate::{ClearColor, FrameSize, Result, SoftwareBuffer};

pub(super) fn render_image(
    image: &RgbaImage,
    size: FrameSize,
    treatment: BackgroundTreatment,
) -> Result<SoftwareBuffer> {
    let (scaled_width, scaled_height) = cover_dimensions(
        image.width(),
        image.height(),
        size.width.max(1),
        size.height.max(1),
    );
    let resized = image::imageops::resize(image, scaled_width, scaled_height, FilterType::Triangle);
    let crop_x = (scaled_width.saturating_sub(size.width)) / 2;
    let crop_y = (scaled_height.saturating_sub(size.height)) / 2;
    let cropped =
        image::imageops::crop_imm(&resized, crop_x, crop_y, size.width, size.height).to_image();
    let cropped = if treatment.blur_radius > 0 {
        image::imageops::blur(&cropped, f32::from(treatment.blur_radius.min(12)))
    } else {
        cropped
    };
    let mut buffer = SoftwareBuffer::new(size)?;

    for (target, pixel) in buffer
        .pixels_mut()
        .chunks_exact_mut(4)
        .zip(cropped.pixels())
    {
        target.copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Ok(buffer)
}

pub(super) fn render_gradient(
    size: FrameSize,
    gradient: BackgroundGradient,
) -> Result<SoftwareBuffer> {
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
