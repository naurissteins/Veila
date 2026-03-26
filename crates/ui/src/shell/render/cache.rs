use veila_renderer::text::{TextBlock, TextStyle, fit_wrapped_text};

use super::{layout::SceneMetrics, model::SceneTextBlocks};

#[derive(Debug, Clone, Default)]
pub(crate) struct TextLayoutCache {
    pub(super) clock: CachedTextBlock,
    pub(super) date: CachedTextBlock,
    pub(super) username: CachedTextBlock,
    pub(super) placeholder: CachedTextBlock,
    pub(super) revealed_secret: CachedTextBlock,
    pub(super) status: CachedTextBlock,
}

#[derive(Debug, Clone, Default)]
pub(super) struct CachedTextBlock {
    pub(super) key: Option<CachedTextKey>,
    pub(super) block: Option<TextBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CachedTextKey {
    pub(super) text: String,
    pub(super) style: TextStyle,
    pub(super) max_width: u32,
    pub(super) min_scale: u32,
}

pub(super) struct SceneTextInputs<'a> {
    pub(super) clock_text: &'a str,
    pub(super) clock_style: TextStyle,
    pub(super) date_text: &'a str,
    pub(super) date_style: TextStyle,
    pub(super) username_text: Option<&'a str>,
    pub(super) username_style: TextStyle,
    pub(super) placeholder_text: &'a str,
    pub(super) placeholder_style: TextStyle,
    pub(super) status_text: Option<&'a str>,
    pub(super) status_style: TextStyle,
    pub(super) metrics: SceneMetrics,
}

impl TextLayoutCache {
    pub(super) fn scene_text_blocks(&mut self, inputs: SceneTextInputs<'_>) -> SceneTextBlocks {
        SceneTextBlocks {
            clock: self.clock.resolve(
                inputs.clock_text,
                inputs.clock_style,
                inputs.metrics.clock_width,
                3,
            ),
            date: self.date.resolve(
                inputs.date_text,
                inputs.date_style,
                inputs.metrics.clock_width,
                1,
            ),
            username: self.username.resolve_optional(
                inputs.username_text,
                inputs.username_style,
                inputs.metrics.content_width,
                1,
            ),
            placeholder: self.placeholder.resolve(
                inputs.placeholder_text,
                inputs.placeholder_style,
                inputs.metrics.input_width.saturating_sub(48) as u32,
                1,
            ),
            status: self.status.resolve_optional(
                inputs.status_text,
                inputs.status_style,
                inputs.metrics.content_width,
                1,
            ),
        }
    }

    pub(super) fn revealed_secret_block(
        &mut self,
        secret: &str,
        style: TextStyle,
        max_width: u32,
    ) -> TextBlock {
        self.revealed_secret.resolve(secret, style, max_width, 1)
    }
}

impl CachedTextBlock {
    fn resolve(
        &mut self,
        text: &str,
        style: TextStyle,
        max_width: u32,
        min_scale: u32,
    ) -> TextBlock {
        let key = CachedTextKey {
            text: text.to_string(),
            style: style.clone(),
            max_width,
            min_scale,
        };

        if self.key.as_ref() == Some(&key)
            && let Some(block) = self.block.as_ref()
        {
            return block.clone();
        }

        let block = fit_wrapped_text(text, style, max_width, min_scale);
        self.key = Some(key);
        self.block = Some(block.clone());
        block
    }

    fn resolve_optional(
        &mut self,
        text: Option<&str>,
        style: TextStyle,
        max_width: u32,
        min_scale: u32,
    ) -> Option<TextBlock> {
        let text = text?;
        Some(self.resolve(text, style, max_width, min_scale))
    }
}
