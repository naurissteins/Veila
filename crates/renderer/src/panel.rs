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

/// Measurements needed to lay out panel body content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelBodyMetrics {
    pub hint_height: i32,
    pub status_height: Option<i32>,
}

/// Shared spacing and sizing for the panel body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelBodyStyle {
    pub horizontal_padding: i32,
    pub hint_to_input_gap: i32,
    pub input_height: i32,
    pub input_to_progress_gap: i32,
    pub progress_height: i32,
    pub progress_to_status_gap: i32,
    pub footer_padding: i32,
}

impl PanelBodyStyle {
    /// Creates a panel body style with Kwylock defaults.
    pub const fn new() -> Self {
        Self {
            horizontal_padding: 32,
            hint_to_input_gap: 22,
            input_height: 38,
            input_to_progress_gap: 24,
            progress_height: 6,
            progress_to_status_gap: 22,
            footer_padding: 28,
        }
    }

    /// Returns the body content width inside a panel rectangle.
    pub fn content_width(self, panel_rect: Rect) -> u32 {
        panel_rect
            .width
            .saturating_sub(self.horizontal_padding * 2)
            .max(0) as u32
    }
}

impl Default for PanelBodyStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// Layout values exposed to panel body rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelBodyLayout {
    pub hint_y: i32,
    pub input_rect: Rect,
    pub progress_rect: Rect,
    pub status_y: Option<i32>,
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

/// Returns the total panel height for the current body metrics.
pub fn measure_panel_height(
    header: PanelHeaderStyle,
    body: PanelBodyStyle,
    metrics: PanelBodyMetrics,
) -> i32 {
    let status_height = metrics
        .status_height
        .map(|height| body.progress_to_status_gap + height)
        .unwrap_or(0);

    header.content_offset_y
        + metrics.hint_height
        + body.hint_to_input_gap
        + body.input_height
        + body.input_to_progress_gap
        + body.progress_height
        + status_height
        + body.footer_padding
}

/// Lays out the panel body from measured content.
pub fn layout_panel_body(
    panel_rect: Rect,
    header: PanelHeaderLayout,
    body: PanelBodyStyle,
    metrics: PanelBodyMetrics,
) -> PanelBodyLayout {
    let input_rect = Rect::new(
        panel_rect.x + body.horizontal_padding,
        header.content_y + metrics.hint_height + body.hint_to_input_gap,
        panel_rect
            .width
            .saturating_sub(body.horizontal_padding * 2)
            .max(0),
        body.input_height,
    );
    let progress_rect = Rect::new(
        input_rect.x,
        input_rect.y + input_rect.height + body.input_to_progress_gap,
        input_rect.width,
        body.progress_height,
    );
    let status_y = metrics
        .status_height
        .map(|_| progress_rect.y + progress_rect.height + body.progress_to_status_gap);

    PanelBodyLayout {
        hint_y: header.content_y,
        input_rect,
        progress_rect,
        status_y,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PanelBodyMetrics, PanelBodyStyle, PanelHeaderStyle, draw_panel_header, layout_panel_body,
        measure_panel_height,
    };
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

    #[test]
    fn measures_panel_height_from_body_metrics() {
        let height = measure_panel_height(
            PanelHeaderStyle::new(ClearColor::opaque(255, 255, 255)),
            PanelBodyStyle::new(),
            PanelBodyMetrics {
                hint_height: 16,
                status_height: Some(20),
            },
        );

        assert_eq!(height, 210);
    }

    #[test]
    fn lays_out_panel_body_regions() {
        let layout = layout_panel_body(
            Rect::new(40, 50, 320, 186),
            draw_panel_header(
                &mut SoftwareBuffer::new(FrameSize::new(400, 300)).expect("buffer"),
                Rect::new(40, 50, 320, 186),
                PanelHeaderStyle::new(ClearColor::opaque(255, 255, 255)),
            ),
            PanelBodyStyle::new(),
            PanelBodyMetrics {
                hint_height: 16,
                status_height: Some(20),
            },
        );

        assert_eq!(layout.hint_y, 84);
        assert_eq!(layout.input_rect, Rect::new(72, 122, 256, 38));
        assert_eq!(layout.progress_rect, Rect::new(72, 184, 256, 6));
        assert_eq!(layout.status_y, Some(212));
    }
}
