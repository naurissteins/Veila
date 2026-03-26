use super::BackgroundTreatment;
use crate::SoftwareBuffer;

pub(super) fn apply_treatment(buffer: &mut SoftwareBuffer, treatment: BackgroundTreatment) {
    if treatment.dim_strength > 0 {
        let multiplier = 255u16.saturating_sub(alpha_from_percent(treatment.dim_strength));
        for pixel in buffer.pixels_mut().chunks_exact_mut(4) {
            pixel[0] = ((u16::from(pixel[0]) * multiplier + 127) / 255) as u8;
            pixel[1] = ((u16::from(pixel[1]) * multiplier + 127) / 255) as u8;
            pixel[2] = ((u16::from(pixel[2]) * multiplier + 127) / 255) as u8;
        }
    }

    if let Some(tint) = treatment.tint {
        let tint_alpha = effective_tint_alpha(tint.alpha, treatment.tint_opacity);
        if tint_alpha > 0 {
            let tint = tint.with_alpha(tint_alpha as u8).to_argb8888_bytes();
            let tint_alpha = u16::from(tint[3]);
            let inverse_alpha = u16::from(u8::MAX) - tint_alpha;
            for pixel in buffer.pixels_mut().chunks_exact_mut(4) {
                pixel[0] = blend_component(pixel[0], tint[0], inverse_alpha);
                pixel[1] = blend_component(pixel[1], tint[1], inverse_alpha);
                pixel[2] = blend_component(pixel[2], tint[2], inverse_alpha);
                pixel[3] = blend_component(pixel[3], tint[3], inverse_alpha);
            }
        }
    }
}

fn alpha_from_percent(percent: u8) -> u16 {
    let clamped = percent.min(100);
    (u16::from(clamped) * 255 + 50) / 100
}

fn effective_tint_alpha(base_alpha: u8, opacity_percent: u8) -> u16 {
    let percent_alpha = alpha_from_percent(opacity_percent);
    if percent_alpha == 0 {
        if base_alpha == u8::MAX {
            0
        } else {
            u16::from(base_alpha)
        }
    } else {
        (u16::from(base_alpha) * percent_alpha + 127) / 255
    }
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}
