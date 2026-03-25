use crate::{
    ClearColor, SoftwareBuffer,
    shape::{CircleStyle, Rect, draw_circle},
};

/// Styling for a masked input row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskedInputStyle {
    pub bullet: ClearColor,
    pub bullet_size: i32,
    pub spacing: i32,
    pub horizontal_padding: i32,
}

impl MaskedInputStyle {
    /// Creates a masked input style with Veila defaults.
    pub const fn new(bullet: ClearColor) -> Self {
        Self {
            bullet,
            bullet_size: 7,
            spacing: 16,
            horizontal_padding: 22,
        }
    }
}

/// Draws a masked input row with left-aligned bullets.
pub fn draw_masked_input(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    secret_len: usize,
    _focused: bool,
    style: MaskedInputStyle,
) {
    if rect.is_empty() || secret_len == 0 {
        return;
    }

    let bullet_size = style.bullet_size.max(1);
    let spacing = style.spacing.max(bullet_size);
    let visible = ((rect.width - style.horizontal_padding * 2) / spacing).max(1) as usize;
    let bullet_count = secret_len.min(visible);
    let start_x = rect.x + style.horizontal_padding.max(0);
    let bullet_y = rect.y + (rect.height - bullet_size) / 2;

    for index in 0..bullet_count {
        let center_x = start_x + index as i32 * spacing + bullet_size / 2;
        draw_circle(
            buffer,
            center_x,
            bullet_y + bullet_size / 2,
            bullet_size / 2,
            CircleStyle::new(style.bullet),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{MaskedInputStyle, draw_masked_input};
    use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

    #[test]
    fn leaves_empty_masked_input_unchanged() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 40)).expect("buffer");
        draw_masked_input(
            &mut buffer,
            Rect::new(0, 0, 120, 40),
            0,
            true,
            MaskedInputStyle::new(ClearColor::opaque(255, 255, 255)),
        );

        assert!(buffer.pixels().iter().all(|byte| *byte == 0));
    }

    #[test]
    fn renders_masked_bullets() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 40)).expect("buffer");
        draw_masked_input(
            &mut buffer,
            Rect::new(0, 0, 120, 40),
            4,
            true,
            MaskedInputStyle::new(ClearColor::opaque(255, 255, 255)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn left_aligns_bullets_to_padding() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 40)).expect("buffer");
        let style = MaskedInputStyle::new(ClearColor::opaque(255, 255, 255));
        draw_masked_input(&mut buffer, Rect::new(0, 0, 120, 40), 4, true, style);

        let width = buffer.size().width as usize;
        let first_drawn_x = buffer
            .pixels()
            .chunks_exact(4)
            .enumerate()
            .find_map(|(index, pixel)| {
                let alpha = pixel[3];
                (alpha != 0).then_some(index % width)
            })
            .expect("rendered pixel");

        assert!(first_drawn_x <= (style.horizontal_padding + 2) as usize);
    }
}
