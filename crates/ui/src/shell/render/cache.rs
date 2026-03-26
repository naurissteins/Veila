use veila_common::WeatherAlignment;
use veila_renderer::{
    icon::WeatherIcon,
    text::{TextBlock, TextStyle, fit_wrapped_text},
};

use super::{
    layout::SceneMetrics,
    model::{SceneTextBlocks, SceneWeatherBlocks},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct TextLayoutCache {
    pub(super) clock: CachedTextBlock,
    pub(super) date: CachedTextBlock,
    pub(super) username: CachedTextBlock,
    pub(super) placeholder: CachedTextBlock,
    pub(super) revealed_secret: CachedTextBlock,
    pub(super) status: CachedTextBlock,
    pub(super) weather_temperature: CachedTextBlock,
    pub(super) weather_location: CachedTextBlock,
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
    pub(super) weather_temperature_text: Option<&'a str>,
    pub(super) weather_temperature_style: TextStyle,
    pub(super) weather_location_text: Option<&'a str>,
    pub(super) weather_location_style: TextStyle,
    pub(super) weather_icon: Option<WeatherIcon>,
    pub(super) weather_icon_size: Option<i32>,
    pub(super) weather_icon_gap: Option<i32>,
    pub(super) weather_location_gap: Option<i32>,
    pub(super) weather_icon_opacity: Option<u8>,
    pub(super) weather_horizontal_padding: Option<i32>,
    pub(super) weather_alignment: WeatherAlignment,
    pub(super) weather_left_offset: Option<i32>,
    pub(super) weather_bottom_offset: Option<i32>,
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
            weather: match (
                inputs.weather_temperature_text,
                inputs.weather_location_text,
                inputs.weather_icon,
            ) {
                (Some(temperature), Some(location), Some(icon)) => {
                    let temperature = self.weather_temperature.resolve(
                        temperature,
                        inputs.weather_temperature_style,
                        inputs.metrics.content_width,
                        1,
                    );
                    let location = self.weather_location.resolve(
                        location,
                        inputs.weather_location_style,
                        inputs.metrics.content_width,
                        1,
                    );
                    let derived_icon_size =
                        SceneWeatherBlocks::clamped_icon_size(temperature.height as i32 + 6);

                    Some(SceneWeatherBlocks {
                        temperature,
                        location,
                        icon,
                        alignment: inputs.weather_alignment,
                        icon_opacity: inputs.weather_icon_opacity,
                        horizontal_padding: inputs
                            .weather_horizontal_padding
                            .unwrap_or(48)
                            .clamp(0, 512),
                        left_offset: inputs.weather_left_offset.unwrap_or(0).clamp(-512, 512),
                        bottom_offset: inputs.weather_bottom_offset.unwrap_or(0).clamp(-512, 512),
                        icon_size: inputs.weather_icon_size.map_or(derived_icon_size, |size| {
                            SceneWeatherBlocks::clamped_icon_size(size)
                        }),
                        icon_gap: inputs.weather_icon_gap.map_or(
                            SceneWeatherBlocks::default_icon_gap(),
                            SceneWeatherBlocks::clamped_icon_gap,
                        ),
                        location_gap: inputs.weather_location_gap.map_or(
                            SceneWeatherBlocks::default_location_gap(),
                            SceneWeatherBlocks::clamped_location_gap,
                        ),
                    })
                }
                _ => None,
            },
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
