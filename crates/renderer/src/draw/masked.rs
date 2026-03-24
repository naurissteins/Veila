use crate::{
    ClearColor, SoftwareBuffer,
    shape::{CircleStyle, PillStyle, Rect, draw_circle, draw_pill, fill_rect},
};

/// Styling for a masked input row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskedInputStyle {
    pub bullet: ClearColor,
    pub placeholder: ClearColor,
    pub caret: ClearColor,
    pub bullet_size: i32,
    pub spacing: i32,
    pub caret_width: i32,
    pub horizontal_padding: i32,
    pub caret_vertical_inset: i32,
    pub placeholder_height: i32,
}

impl MaskedInputStyle {
    /// Creates a masked input style with Veila defaults.
    pub const fn new(bullet: ClearColor, placeholder: ClearColor, caret: ClearColor) -> Self {
        Self {
            bullet,
            placeholder,
            caret,
            bullet_size: 8,
            spacing: 18,
            caret_width: 2,
            horizontal_padding: 24,
            caret_vertical_inset: 12,
            placeholder_height: 6,
        }
    }
}

/// Draws a masked input row with bullets, placeholder, and optional caret.
pub fn draw_masked_input(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    secret_len: usize,
    focused: bool,
    style: MaskedInputStyle,
) {
    if rect.is_empty() {
        return;
    }

    if secret_len == 0 {
        draw_empty_input(buffer, rect, focused, style);
        return;
    }

    let bullet_size = style.bullet_size.max(1);
    let spacing = style.spacing.max(bullet_size);
    let visible = ((rect.width - style.horizontal_padding * 2) / spacing).max(1) as usize;
    let bullet_count = secret_len.min(visible);
    let row_width = bullet_count as i32 * bullet_size
        + bullet_count.saturating_sub(1) as i32 * (spacing - bullet_size);
    let start_x = rect.x + ((rect.width - row_width) / 2).max(style.horizontal_padding);
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

    if focused {
        let cursor_x = start_x + bullet_count as i32 * spacing + 4;
        fill_rect(
            buffer,
            Rect::new(
                cursor_x,
                rect.y + style.caret_vertical_inset,
                style.caret_width.max(1),
                (rect.height - style.caret_vertical_inset * 2).max(1),
            ),
            style.caret,
        );
    }
}

fn draw_empty_input(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    focused: bool,
    style: MaskedInputStyle,
) {
    draw_pill(
        buffer,
        Rect::new(
            rect.x + style.horizontal_padding,
            rect.y + (rect.height / 2) - (style.placeholder_height / 2),
            (rect.width / 3).max(style.placeholder_height * 6),
            style.placeholder_height,
        ),
        PillStyle::new(style.placeholder),
    );

    if focused {
        fill_rect(
            buffer,
            Rect::new(
                rect.x + style.horizontal_padding,
                rect.y + style.caret_vertical_inset + 1,
                style.caret_width.max(1),
                (rect.height - (style.caret_vertical_inset + 1) * 2).max(1),
            ),
            style.caret,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{MaskedInputStyle, draw_masked_input};
    use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

    #[test]
    fn renders_empty_masked_input() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 40)).expect("buffer");
        draw_masked_input(
            &mut buffer,
            Rect::new(0, 0, 120, 40),
            0,
            true,
            MaskedInputStyle::new(
                ClearColor::opaque(255, 255, 255),
                ClearColor::opaque(72, 82, 108),
                ClearColor::opaque(96, 164, 255),
            ),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn renders_masked_bullets() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(120, 40)).expect("buffer");
        draw_masked_input(
            &mut buffer,
            Rect::new(0, 0, 120, 40),
            4,
            true,
            MaskedInputStyle::new(
                ClearColor::opaque(255, 255, 255),
                ClearColor::opaque(72, 82, 108),
                ClearColor::opaque(96, 164, 255),
            ),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
