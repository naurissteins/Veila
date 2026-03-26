use image::{RgbaImage, imageops::FilterType};

use super::BackgroundTreatment;
use crate::{FrameSize, Result, SoftwareBuffer};

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
