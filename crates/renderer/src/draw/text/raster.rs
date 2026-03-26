use cosmic_text::{Buffer, Wrap};

use crate::SoftwareBuffer;

use super::{
    ClearColor, TextStyle, context::FONT_CONTEXT, font_metrics, modulate_alpha, text_attrs,
    text_color,
};

pub(super) fn draw_text_lines(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    lines: &[String],
    style: TextStyle,
    color: ClearColor,
) {
    if lines.iter().all(String::is_empty) {
        return;
    }

    let text = lines.join("\n");
    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let super::context::FontContext {
            font_system,
            swash_cache,
        } = &mut *context;
        let mut cosmic_buffer = Buffer::new(font_system, font_metrics(&style));
        cosmic_buffer.set_wrap(font_system, Wrap::None);
        cosmic_buffer.set_size(font_system, None, None);
        let attrs = text_attrs(&style);
        cosmic_buffer.set_text(font_system, &text, &attrs, cosmic_text::Shaping::Advanced);
        cosmic_buffer.shape_until_scroll(font_system, true);

        cosmic_buffer.draw(
            font_system,
            swash_cache,
            text_color(color),
            |pixel_x, pixel_y, width, height, pixel_color| {
                let pixel_color = modulate_alpha(pixel_color, color.alpha);
                for offset_y in 0..height as i32 {
                    for offset_x in 0..width as i32 {
                        blend_pixel(
                            buffer,
                            x + pixel_x + offset_x,
                            y + pixel_y + offset_y,
                            pixel_color,
                        );
                    }
                }
            },
        );
    });
}

fn blend_pixel(buffer: &mut SoftwareBuffer, x: i32, y: i32, color: cosmic_text::Color) {
    let size = buffer.size();
    if x < 0 || y < 0 || x >= size.width as i32 || y >= size.height as i32 {
        return;
    }

    let src_alpha = color.a() as u16;
    if src_alpha == 0 {
        return;
    }

    let src_red = premultiply(color.r(), color.a());
    let src_green = premultiply(color.g(), color.a());
    let src_blue = premultiply(color.b(), color.a());
    let stride = size.width as usize * 4;
    let offset = y as usize * stride + x as usize * 4;
    let pixels = buffer.pixels_mut();

    if src_alpha == u16::from(u8::MAX) {
        pixels[offset] = src_blue;
        pixels[offset + 1] = src_green;
        pixels[offset + 2] = src_red;
        pixels[offset + 3] = color.a();
        return;
    }

    let inverse_alpha = u16::from(u8::MAX) - src_alpha;
    pixels[offset] = blend_component(pixels[offset], src_blue, inverse_alpha);
    pixels[offset + 1] = blend_component(pixels[offset + 1], src_green, inverse_alpha);
    pixels[offset + 2] = blend_component(pixels[offset + 2], src_red, inverse_alpha);
    pixels[offset + 3] = blend_component(pixels[offset + 3], color.a(), inverse_alpha);
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

fn blend_component(dst: u8, src: u8, inverse_alpha: u16) -> u8 {
    let blended = u16::from(src) + ((u16::from(dst) * inverse_alpha + 127) / 255);
    blended.min(u16::from(u8::MAX)) as u8
}
