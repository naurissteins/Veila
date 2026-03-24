use tiny_skia::{Color, Pixmap};

use crate::{ClearColor, SoftwareBuffer};

pub(crate) fn color(color: ClearColor) -> Color {
    Color::from_rgba8(color.red, color.green, color.blue, color.alpha)
}

pub(crate) fn draw_overlay(
    buffer: &mut SoftwareBuffer,
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
    painter: impl FnOnce(&mut Pixmap),
) {
    if width == 0 || height == 0 {
        return;
    }

    let Some(mut overlay) = Pixmap::new(width, height) else {
        return;
    };
    painter(&mut overlay);
    blend_pixmap(buffer, origin_x, origin_y, &overlay);
}

fn blend_pixmap(buffer: &mut SoftwareBuffer, origin_x: i32, origin_y: i32, overlay: &Pixmap) {
    let target_width = buffer.size().width as i32;
    let target_height = buffer.size().height as i32;
    let overlay_width = overlay.width() as i32;
    let overlay_height = overlay.height() as i32;

    let left = origin_x.clamp(0, target_width);
    let top = origin_y.clamp(0, target_height);
    let right = (origin_x + overlay_width).clamp(0, target_width);
    let bottom = (origin_y + overlay_height).clamp(0, target_height);

    if left >= right || top >= bottom {
        return;
    }

    let overlay_stride = overlay.width() as usize * 4;
    let buffer_stride = buffer.size().width as usize * 4;
    let pixels = buffer.pixels_mut();

    for y in top..bottom {
        let overlay_y = (y - origin_y) as usize;
        let buffer_y = y as usize;
        for x in left..right {
            let overlay_x = (x - origin_x) as usize;
            let buffer_x = x as usize;

            let src_offset = overlay_y * overlay_stride + overlay_x * 4;
            let dst_offset = buffer_y * buffer_stride + buffer_x * 4;
            let src = &overlay.data()[src_offset..src_offset + 4];
            blend_pixel(&mut pixels[dst_offset..dst_offset + 4], src);
        }
    }
}

fn blend_pixel(dst: &mut [u8], src: &[u8]) {
    let src_alpha = src[3] as u16;
    if src_alpha == 0 {
        return;
    }

    if src_alpha == u16::from(u8::MAX) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = src[3];
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    dst[0] = blend_component(dst[0], src[2], inverse_alpha);
    dst[1] = blend_component(dst[1], src[1], inverse_alpha);
    dst[2] = blend_component(dst[2], src[0], inverse_alpha);
    dst[3] = blend_component(dst[3], src[3], inverse_alpha);
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::draw_overlay;
    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    #[test]
    fn blends_translucent_overlay_into_buffer() {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(2, 2), ClearColor::opaque(0, 0, 0)).unwrap();

        draw_overlay(&mut buffer, 0, 0, 2, 2, |overlay| {
            overlay.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 128));
        });

        assert!(buffer.pixels()[2] > 0);
    }
}
