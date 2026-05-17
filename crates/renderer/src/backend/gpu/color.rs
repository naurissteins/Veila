use crate::SoftwareBuffer;

pub(super) fn solid_clear_pixel_from_buffer(buffer: &SoftwareBuffer) -> Option<[u8; 4]> {
    let Some(pixel) = buffer.pixels().chunks_exact(4).next() else {
        return Some([0, 0, 0, 255]);
    };

    if !buffer.pixels().chunks_exact(4).all(|chunk| chunk == pixel) {
        return None;
    }

    Some([pixel[0], pixel[1], pixel[2], pixel[3]])
}

pub(super) fn clear_color_from_bgra_pixel(
    pixel: [u8; 4],
    surface_format: wgpu::TextureFormat,
) -> wgpu::Color {
    let component = if surface_format.is_srgb() {
        srgb_component_to_linear
    } else {
        byte_to_unit
    };

    wgpu::Color {
        r: component(pixel[2]),
        g: component(pixel[1]),
        b: component(pixel[0]),
        a: byte_to_unit(pixel[3]),
    }
}

fn srgb_component_to_linear(value: u8) -> f64 {
    let value = byte_to_unit(value);
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn byte_to_unit(value: u8) -> f64 {
    f64::from(value) / 255.0
}
