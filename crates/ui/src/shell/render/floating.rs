use veila_renderer::{FrameSize, shape::Rect, text::TextBlock};

use super::{RenderContext, layout::SceneMetrics};
use crate::shell::theme::WidgetPosition;

impl RenderContext<'_> {
    pub(super) fn positioned_text_block(
        &self,
        size: FrameSize,
        position: Option<WidgetPosition>,
        block: Option<&TextBlock>,
    ) -> Option<(Rect, TextBlock)> {
        position.zip(block).map(|(position, block)| {
            let rect =
                self.positioned_rect(size, position, block.width as i32, block.height as i32);
            (rect, block.clone())
        })
    }

    pub(super) fn floating_status_rect(
        &self,
        size: FrameSize,
        metrics: SceneMetrics,
        block: &TextBlock,
    ) -> Option<Rect> {
        if let Some(position) = self.theme.status_position {
            return Some(self.positioned_rect(
                size,
                position,
                block.width as i32,
                block.height as i32,
            ));
        }

        let position = self.theme.input_position?;
        let input = self.positioned_rect(size, position, metrics.input_width, metrics.input_height);
        let x = input.x + (input.width - block.width as i32) / 2;
        // Bottom-anchored input keeps its feedback above the field.
        let y = if position.valign == veila_common::VerticalAlign::Bottom {
            input.y - 14 - block.height as i32
        } else {
            input.y + input.height + 14
        };
        Some(Rect::new(x, y, block.width as i32, block.height as i32))
    }
}
