use serde::{Deserialize, Serialize};

use super::{RgbColor, WidgetPositionConfig};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct InputVisualConfig {
    pub placeholder: Option<String>,
    pub reveal_on_interaction: Option<bool>,
    pub reveal_mode: Option<InputRevealMode>,
    pub reveal_hint: Option<String>,
    pub font_family: Option<String>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub font_size: Option<u16>,
    pub background_color: Option<RgbColor>,
    pub border_color: Option<RgbColor>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub radius: Option<u16>,
    pub border_width: Option<u16>,
    pub mask_color: Option<RgbColor>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for InputVisualConfig {
    fn default() -> Self {
        Self {
            placeholder: Some(String::from(DEFAULT_INPUT_PLACEHOLDER)),
            reveal_on_interaction: Some(false),
            reveal_mode: Some(InputRevealMode::Input),
            reveal_hint: Some(String::from(DEFAULT_REVEAL_HINT)),
            font_family: Some(super::default_google_sans_flex_font_family()),
            font_weight: Some(400),
            font_style: Some(FontStyle::Normal),
            font_size: Some(16),
            background_color: Some(RgbColor::rgba(255, 255, 255, 10)),
            border_color: Some(RgbColor::rgba(255, 255, 255, 0)),
            width: Some(310),
            height: Some(54),
            radius: Some(10),
            border_width: Some(0),
            mask_color: Some(RgbColor::rgb(255, 255, 255)),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Center),
                valign: Some(super::VerticalAlign::Center),
                x: Some(0),
                y: Some(80),
                relative_to: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum FontStyle {
    #[default]
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "italic")]
    Italic,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputRevealMode {
    #[default]
    #[serde(rename = "input")]
    Input,
    #[serde(rename = "full")]
    Full,
}

const fn default_input_color() -> RgbColor {
    RgbColor::rgb(13, 18, 28)
}

const DEFAULT_INPUT_PLACEHOLDER: &str = "Password";
const DEFAULT_REVEAL_HINT: &str = "Press any key or click to continue";
const MAX_REVEAL_HINT_CHARS: usize = 160;

pub(crate) fn sanitized_reveal_hint(hint: Option<&str>) -> String {
    let trimmed = hint.map(str::trim).filter(|value| !value.is_empty());
    trimmed
        .unwrap_or(DEFAULT_REVEAL_HINT)
        .chars()
        .take(MAX_REVEAL_HINT_CHARS)
        .collect()
}

impl super::VisualConfig {
    pub fn input_placeholder(&self) -> String {
        self.input
            .placeholder
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_INPUT_PLACEHOLDER)
            .to_owned()
    }

    pub fn input_background_color(&self) -> RgbColor {
        self.input
            .background_color
            .unwrap_or_else(default_input_color)
    }

    pub fn input_border_color(&self) -> RgbColor {
        self.input
            .border_color
            .unwrap_or(RgbColor::rgba(255, 255, 255, 0))
    }

    pub fn input_font_family(&self) -> Option<&str> {
        self.input
            .font_family
            .as_deref()
            .or(Some(super::DEFAULT_INPUT_FONT_FAMILY))
    }

    pub fn input_reveal_on_interaction(&self) -> bool {
        self.input.reveal_on_interaction.unwrap_or(false)
    }

    pub fn input_reveal_mode(&self) -> InputRevealMode {
        self.input.reveal_mode.unwrap_or_default()
    }

    pub fn input_reveal_hint(&self) -> String {
        sanitized_reveal_hint(self.input.reveal_hint.as_deref())
    }

    pub fn input_position(&self) -> WidgetPositionConfig {
        self.input.position.clone()
    }

    pub fn input_font_weight(&self) -> Option<u16> {
        self.input.font_weight.or(Some(400))
    }

    pub fn input_font_style(&self) -> Option<FontStyle> {
        self.input.font_style.or(Some(FontStyle::Normal))
    }

    pub fn input_font_size(&self) -> Option<u16> {
        self.input.font_size.or(Some(16))
    }

    pub fn input_width(&self) -> Option<u16> {
        self.input.width.or(Some(310))
    }

    pub fn input_height(&self) -> Option<u16> {
        self.input.height.or(Some(54))
    }

    pub fn input_radius(&self) -> u16 {
        self.input.radius.unwrap_or(10)
    }

    pub fn input_border_width(&self) -> Option<u16> {
        self.input.border_width.or(Some(0))
    }

    pub fn input_mask_color(&self) -> Option<RgbColor> {
        self.input.mask_color.or(Some(RgbColor::rgb(255, 255, 255)))
    }
}
