use serde::{Deserialize, Serialize};

use super::RgbColor;

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
pub struct InputVisualConfig {
    #[serde(default)]
    pub alignment: Option<InputAlignment>,
    #[serde(default)]
    pub center_in_layer: Option<bool>,
    #[serde(default)]
    pub horizontal_padding: Option<u16>,
    #[serde(default)]
    pub vertical_padding: Option<u16>,
    #[serde(default)]
    pub offset_x: Option<i16>,
    #[serde(default)]
    pub offset_y: Option<i16>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<FontStyle>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_opacity: Option<u8>,
    #[serde(default)]
    pub border_color: Option<RgbColor>,
    #[serde(default)]
    pub border_opacity: Option<u8>,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub border_width: Option<u16>,
    #[serde(default)]
    pub mask_color: Option<RgbColor>,
}

impl Default for InputVisualConfig {
    fn default() -> Self {
        Self {
            alignment: Some(InputAlignment::CenterCenter),
            center_in_layer: Some(false),
            horizontal_padding: None,
            vertical_padding: None,
            offset_x: Some(0),
            offset_y: Some(0),
            font_family: Some(super::default_google_sans_flex_font_family()),
            font_weight: Some(400),
            font_style: Some(FontStyle::Normal),
            font_size: Some(2),
            background_color: Some(RgbColor::rgb(255, 255, 255)),
            background_opacity: Some(5),
            border_color: Some(RgbColor::rgb(255, 255, 255)),
            border_opacity: Some(0),
            width: Some(310),
            height: Some(54),
            radius: Some(10),
            border_width: Some(0),
            mask_color: Some(RgbColor::rgb(255, 255, 255)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputAlignment {
    #[default]
    #[serde(rename = "center-center")]
    CenterCenter,
    #[serde(rename = "center-right")]
    CenterRight,
    #[serde(rename = "center-left")]
    CenterLeft,
    #[serde(rename = "top-center")]
    TopCenter,
    #[serde(rename = "top-right")]
    TopRight,
    #[serde(rename = "top-left")]
    TopLeft,
    #[serde(rename = "bottom-center")]
    BottomCenter,
    #[serde(rename = "bottom-right")]
    BottomRight,
    #[serde(rename = "bottom-left")]
    BottomLeft,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum FontStyle {
    #[default]
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "italic")]
    Italic,
}

const fn default_input_color() -> RgbColor {
    RgbColor::rgb(13, 18, 28)
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

    pub fn input_background_opacity(&self) -> Option<u8> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_opacity,
            InputVisualEntry::Section(config) => config.background_opacity.or(self.input_opacity),
        }
    }

    pub fn input_border_color(&self) -> RgbColor {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_border,
            InputVisualEntry::Section(config) => config.border_color.unwrap_or(self.input_border),
        }
    }

    pub fn input_border_opacity(&self) -> Option<u8> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_border_opacity,
            InputVisualEntry::Section(config) => {
                config.border_opacity.or(self.input_border_opacity)
            }
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

    pub fn input_alignment(&self) -> InputAlignment {
        match &self.input {
            InputVisualEntry::Color(_) => InputAlignment::default(),
            InputVisualEntry::Section(config) => config.alignment.unwrap_or_default(),
        }
    }

    pub fn input_horizontal_padding(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.horizontal_padding,
        }
    }

    pub fn input_center_in_layer(&self) -> bool {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_center_in_layer.unwrap_or(false),
            InputVisualEntry::Section(config) => config
                .center_in_layer
                .or(self.input_center_in_layer)
                .unwrap_or(false),
        }
    }

    pub fn input_vertical_padding(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.vertical_padding,
        }
    }

    pub fn input_offset_x(&self) -> Option<i16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.offset_x,
        }
    }

    pub fn input_offset_y(&self) -> Option<i16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.offset_y,
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
