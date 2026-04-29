use image::{RgbaImage, imageops, imageops::FilterType};

const DOWNSAMPLE_THRESHOLD_RADIUS: u8 = 6;

pub(crate) fn blur_rgba(image: &RgbaImage, radius: u8, max_radius: u8) -> RgbaImage {
    let radius = radius.min(max_radius);
    if radius == 0 {
        return image.clone();
    }

    let factor = blur_downsample_factor(image.width(), image.height(), radius);
    if factor <= 1 {
        return imageops::blur(image, f32::from(radius));
    }

    let downsampled = imageops::resize(
        image,
        image.width().div_ceil(factor).max(1),
        image.height().div_ceil(factor).max(1),
        FilterType::Triangle,
    );
    let blurred = imageops::blur(&downsampled, f32::from(radius) / factor as f32);
    imageops::resize(
        &blurred,
        image.width().max(1),
        image.height().max(1),
        FilterType::Triangle,
    )
}

fn blur_downsample_factor(width: u32, height: u32, radius: u8) -> u32 {
    if radius < DOWNSAMPLE_THRESHOLD_RADIUS {
        return 1;
    }

    let pixels = u64::from(width) * u64::from(height);
    if radius >= 12 || pixels >= 1_200_000 {
        return 3;
    }

    if radius >= 8 || pixels >= 500_000 {
        return 2;
    }

    1
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::blur_rgba;

    #[test]
    fn preserves_dimensions_for_downsampled_blur() {
        let mut image = RgbaImage::new(17, 11);
        image.put_pixel(3, 4, Rgba([255, 255, 255, 255]));

        let blurred = blur_rgba(&image, 12, 24);

        assert_eq!(blurred.width(), 17);
        assert_eq!(blurred.height(), 11);
    }

    #[test]
    fn changes_pixels_when_blur_radius_is_non_zero() {
        let mut image = RgbaImage::new(9, 9);
        image.put_pixel(4, 4, Rgba([255, 255, 255, 255]));

        let blurred = blur_rgba(&image, 8, 24);

        assert_ne!(blurred.get_pixel(4, 4).0, image.get_pixel(4, 4).0);
    }
}
