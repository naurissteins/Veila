use super::RenderContext;
use crate::shell::theme::VisualLayer;
use veila_common::LayerKind;
use veila_renderer::{
    PixelBuffer,
    shape::{PillStyle, draw_pill},
};

pub(super) const STARTUP_TEXT_BUDGET: usize = 64 * 1024;
pub(super) const MAX_CUSTOM_LAYER_FONT_SIZE_PX: u32 = 512;
const STARTUP_TITLE_BUDGET: usize = 256 * 1024;
const MIN_STARTUP_TITLE_SIZE_PX: usize = 64;
const MAX_STARTUP_TITLE_BYTES: usize = 16;
const STARTUP_WORDMARK_BUDGET: usize = 5 * MAX_CUSTOM_LAYER_FONT_SIZE_PX.pow(2) as usize;
const MIN_STARTUP_WORDMARK_SIZE_PX: usize = 128;

impl RenderContext<'_> {
    pub(super) fn render_layers(
        &self,
        buffer: &mut impl PixelBuffer,
        mut startup_budget: Option<usize>,
    ) -> bool {
        if startup_budget == Some(0) {
            return self.theme.layers.is_empty();
        }

        let mut complete = true;
        let mut title_available = true;
        let mut wordmark_available = true;
        for (index, layer) in self.theme.layers.iter().enumerate() {
            if let Some(remaining) = &mut startup_budget
                && !reserve_startup_text(
                    layer,
                    remaining,
                    &mut title_available,
                    &mut wordmark_available,
                )
            {
                complete = false;
                continue;
            }
            self.render_layer(buffer, index, layer);
        }

        complete
    }

    fn render_layer(&self, buffer: &mut impl PixelBuffer, index: usize, layer: &VisualLayer) {
        let max_text_width = layer
            .width
            .map(|width| width.saturating_sub(layer.padding * 2).max(1) as u32)
            .unwrap_or(4_096);
        let style = self.custom_layer_text_style(layer);
        let block = self.text_layout_cache.borrow_mut().custom_layer_block(
            index,
            &layer.text,
            style,
            max_text_width,
            matches!(layer.kind, LayerKind::Icon),
        );
        let icon_bounds = matches!(layer.kind, LayerKind::Icon)
            .then(|| {
                self.text_layout_cache
                    .borrow_mut()
                    .custom_layer_visible_bounds(index, &layer.text, block.style.clone())
            })
            .flatten();
        let content_width = icon_bounds.map_or(block.width as i32, |bounds| bounds.width());
        let content_height = icon_bounds.map_or(block.height as i32, |bounds| bounds.height());
        let min_width = content_width + layer.padding * 2;
        let min_height = content_height + layer.padding * 2;
        let width = layer.width.unwrap_or(min_width).max(min_width);
        let height = layer.height.unwrap_or(min_height).max(min_height);
        let rect = self.positioned_rect(buffer.size(), layer.position, width, height);

        if let Some(background) = layer.background_color
            && background.alpha > 0
        {
            draw_pill(
                buffer,
                rect,
                PillStyle::new(background).with_radius(layer.radius),
            );
        }

        let (text_x, text_y) = icon_bounds.map_or_else(
            || {
                (
                    rect.x + (rect.width - content_width) / 2,
                    rect.y + (rect.height - content_height) / 2,
                )
            },
            |bounds| {
                (
                    rect.x + rect.width / 2 - (bounds.left + bounds.right) / 2,
                    rect.y + rect.height / 2 - (bounds.top + bounds.bottom) / 2,
                )
            },
        );
        block.draw(buffer, text_x, text_y);
    }
}

fn reserve_startup_text(
    layer: &VisualLayer,
    remaining: &mut usize,
    title_available: &mut bool,
    wordmark_available: &mut bool,
) -> bool {
    if layer.kind != LayerKind::Text || layer.background_color.is_some_and(|color| color.alpha > 0)
    {
        return false;
    }

    // Estimate raster work in device pixels and count small text toward the shaping budget too.
    let font_size = layer.font_size.clamp(16, MAX_CUSTOM_LAYER_FONT_SIZE_PX) as usize;
    let cost = layer
        .text
        .len()
        .saturating_mul(font_size.saturating_mul(font_size));
    // reserve the label budget for smaller layers when a short title can use its own allowance
    if *title_available
        && font_size >= MIN_STARTUP_TITLE_SIZE_PX
        && cost <= STARTUP_TITLE_BUDGET
        && layer.text.len() <= MAX_STARTUP_TITLE_BYTES
        && layer.text.contains(' ')
        && layer
            .text
            .bytes()
            .all(|byte| byte == b' ' || byte.is_ascii_graphic())
    {
        *title_available = false;
        return true;
    }
    if let Some(unused) = remaining.checked_sub(cost) {
        *remaining = unused;
        return true;
    }

    // The separate allowance keeps one large single word from using the label budget.
    if *wordmark_available
        && font_size >= MIN_STARTUP_WORDMARK_SIZE_PX
        && cost <= STARTUP_WORDMARK_BUDGET
        && layer.text.bytes().all(|byte| byte.is_ascii_graphic())
    {
        *wordmark_available = false;
        return true;
    }
    false
}
