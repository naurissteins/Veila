mod auth;
mod header;
mod widgets;

use veila_common::FontStyle as ConfigFontStyle;
use veila_renderer::text::{FontStyle as RendererFontStyle, TextStyle, resolve_font_family};

use super::super::RenderContext;

const MAX_CUSTOM_LAYER_FONT_SIZE_PX: u32 = 512;

impl RenderContext<'_> {
    pub(crate) fn custom_layer_text_style(
        &self,
        layer: &crate::shell::theme::VisualLayer,
    ) -> TextStyle {
        let style = TextStyle::new_px(
            layer.color,
            layer.font_size.clamp(1, MAX_CUSTOM_LAYER_FONT_SIZE_PX),
        )
        .with_line_spacing(0);

        self.apply_font_overrides(
            style,
            self.resolved_font_family(layer.font_family.as_deref()),
            layer.font_weight,
            layer.font_style,
        )
    }

    fn resolved_font_family(&self, family: Option<&str>) -> Option<String> {
        family
            .and_then(resolve_font_family)
            .or_else(|| family.map(str::to_owned))
    }

    fn apply_font_overrides(
        &self,
        style: TextStyle,
        family: Option<String>,
        weight: Option<u16>,
        font_style: Option<ConfigFontStyle>,
    ) -> TextStyle {
        let style = match family {
            Some(family) => style.with_font_family(&family),
            None => style,
        };
        let style = match weight {
            Some(weight) => style.with_font_weight(weight),
            None => style,
        };

        match font_style {
            Some(ConfigFontStyle::Normal) => style.with_font_style(RendererFontStyle::Normal),
            Some(ConfigFontStyle::Italic) => style.with_font_style(RendererFontStyle::Italic),
            None => style,
        }
    }
}
