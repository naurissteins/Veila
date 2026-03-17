use crate::{
    ClearColor, SoftwareBuffer,
    shape::{Rect, fill_rect},
};

/// Styling for a panel header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelHeaderStyle {
    pub accent: ClearColor,
    pub accent_height: i32,
    pub content_offset_y: i32,
}

impl PanelHeaderStyle {
    /// Creates a panel header style with Kwylock defaults.
    pub const fn new(accent: ClearColor) -> Self {
        Self {
            accent,
            accent_height: 6,
            content_offset_y: 34,
        }
    }
}

/// Header layout values exposed to panel content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelHeaderLayout {
    pub content_y: i32,
}

/// Draws a simple panel header and returns content anchors.
pub fn draw_panel_header(
    buffer: &mut SoftwareBuffer,
    panel_rect: Rect,
    style: PanelHeaderStyle,
) -> PanelHeaderLayout {
    fill_rect(
        buffer,
        Rect::new(
            panel_rect.x,
            panel_rect.y,
            panel_rect.width,
            style.accent_height,
        ),
        style.accent,
    );

    PanelHeaderLayout {
        content_y: panel_rect.y + style.content_offset_y,
    }
}

#[cfg(test)]
mod tests {
    use super::{PanelHeaderStyle, draw_panel_header};
    use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::Rect};

    #[test]
    fn draws_panel_header_accent() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(80, 40)).expect("buffer");
        let layout = draw_panel_header(
            &mut buffer,
            Rect::new(10, 10, 60, 20),
            PanelHeaderStyle::new(ClearColor::opaque(255, 255, 255)),
        );

        assert_eq!(layout.content_y, 44);
        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
