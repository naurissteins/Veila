use cosmic_text::{Buffer, Shaping, Wrap};

use super::{
    TextBlock, TextStyle, context::FONT_CONTEXT, extract_run_text, font_metrics, text_attrs,
};

pub(super) fn layout_text_block(
    text: &str,
    style: TextStyle,
    max_width: Option<u32>,
    wrap: Wrap,
) -> TextBlock {
    if text.is_empty() {
        let height = line_height(&style);
        return TextBlock {
            lines: vec![String::new()],
            style,
            width: 0,
            height,
        };
    }

    FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let mut buffer = Buffer::new(&mut context.font_system, font_metrics(&style));
        buffer.set_wrap(&mut context.font_system, wrap);
        buffer.set_size(
            &mut context.font_system,
            max_width.map(|value| value as f32),
            None,
        );
        let attrs = text_attrs(&style);
        buffer.set_text(&mut context.font_system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut context.font_system, true);

        let mut width = 0.0f32;
        let mut bottom = 0.0f32;
        let mut lines = Vec::new();

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            bottom = bottom.max(run.line_top + run.line_height);
            lines.push(extract_run_text(run.text, run.glyphs));
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        let height = bottom.ceil().max(line_height(&style) as f32) as u32;

        TextBlock {
            lines,
            style,
            width: width.ceil().max(0.0) as u32,
            height,
        }
    })
}

pub(super) fn font_size(style: &TextStyle) -> f32 {
    4.0 + style.scale.max(1) as f32 * 6.0
}

pub(super) fn line_height(style: &TextStyle) -> u32 {
    font_size(style).ceil() as u32 + style.line_spacing
}

pub(super) fn scale_component(component: u32, current_scale: u32, next_scale: u32) -> u32 {
    let scaled = component
        .saturating_mul(next_scale)
        .div_ceil(current_scale.max(1));
    scaled.max(next_scale.min(1))
}
