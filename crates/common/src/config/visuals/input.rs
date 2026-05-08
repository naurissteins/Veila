use serde::{Deserialize, Serialize};

use super::{RgbColor, WidgetPositionConfig};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum InputVisualEntry {
    Color(RgbColor),
    Section(InputVisualConfig),
}

impl Default for InputVisualEntry {
    fn default() -> Self {
        Self::Color(default_input_color())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct InputVisualConfig {
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
            reveal_on_interaction: Some(false),
            reveal_mode: Some(InputRevealMode::Input),
            reveal_hint: Some(String::from(DEFAULT_REVEAL_HINT)),
            font_family: Some(super::default_google_sans_flex_font_family()),
            font_weight: Some(400),
            font_style: Some(FontStyle::Normal),
            font_size: Some(16),
            background_color: Some(RgbColor::rgba(255, 255, 255, 13)),
            border_color: Some(RgbColor::rgba(255, 255, 255, 0)),
            width: Some(310),
            height: Some(54),
            radius: Some(10),
            border_width: Some(0),
            mask_color: Some(RgbColor::rgb(255, 255, 255)),
            position: WidgetPositionConfig::default(),
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
    pub fn input_background_color(&self) -> RgbColor {
        match &self.input {
            InputVisualEntry::Color(color) => *color,
            InputVisualEntry::Section(config) => {
                config.background_color.unwrap_or_else(default_input_color)
            }
        }
    }

    pub fn input_border_color(&self) -> RgbColor {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_border,
            InputVisualEntry::Section(config) => config.border_color.unwrap_or(self.input_border),
        }
    }

    pub fn input_width(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_width,
            InputVisualEntry::Section(config) => config.width.or(self.input_width),
        }
    }

    pub fn input_font_family(&self) -> Option<&str> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_font_family.as_deref(),
            InputVisualEntry::Section(config) => config
                .font_family
                .as_deref()
                .or(self.input_font_family.as_deref()),
        }
    }

    pub fn input_reveal_on_interaction(&self) -> bool {
        match &self.input {
            InputVisualEntry::Color(_) => false,
            InputVisualEntry::Section(config) => config.reveal_on_interaction.unwrap_or(false),
        }
    }

    pub fn input_reveal_mode(&self) -> InputRevealMode {
        match &self.input {
            InputVisualEntry::Color(_) => InputRevealMode::Input,
            InputVisualEntry::Section(config) => config.reveal_mode.unwrap_or_default(),
        }
    }

    pub fn input_reveal_hint(&self) -> String {
        match &self.input {
            InputVisualEntry::Color(_) => sanitized_reveal_hint(None),
            InputVisualEntry::Section(config) => {
                sanitized_reveal_hint(config.reveal_hint.as_deref())
            }
        }
    }

    pub fn input_position(&self) -> WidgetPositionConfig {
        match &self.input {
            InputVisualEntry::Color(_) => WidgetPositionConfig::default(),
            InputVisualEntry::Section(config) => config.position.clone(),
        }
    }

    pub fn input_font_weight(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_font_weight,
            InputVisualEntry::Section(config) => config.font_weight.or(self.input_font_weight),
        }
    }

    pub fn input_font_style(&self) -> Option<FontStyle> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_font_style,
            InputVisualEntry::Section(config) => config.font_style.or(self.input_font_style),
        }
    }

    pub fn input_font_size(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_font_size,
            InputVisualEntry::Section(config) => config.font_size.or(self.input_font_size),
        }
    }

    pub fn input_height(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_height,
            InputVisualEntry::Section(config) => config.height.or(self.input_height),
        }
    }

    pub fn input_radius(&self) -> u16 {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_radius,
            InputVisualEntry::Section(config) => config.radius.unwrap_or(self.input_radius),
        }
    }

    pub fn input_border_width(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_border_width,
            InputVisualEntry::Section(config) => config.border_width.or(self.input_border_width),
        }
    }

    pub fn input_mask_color(&self) -> Option<RgbColor> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_mask_color,
            InputVisualEntry::Section(config) => config.mask_color.or(self.input_mask_color),
        }
    }
}
