use serde::{Deserialize, Serialize};

use super::{FontStyle, RgbColor, WidgetPositionConfig};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum StatusDisplayMode {
    #[default]
    #[serde(rename = "inline")]
    Inline,
    #[serde(rename = "external")]
    External,
    #[serde(rename = "hidden")]
    Hidden,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RevealDisplayMode {
    #[default]
    #[serde(rename = "shown")]
    Shown,
    #[serde(rename = "hidden")]
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaceholderVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
}

impl Default for PlaceholderVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgba(255, 255, 255, 230)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevealVisualConfig {
    #[serde(default)]
    pub mode: Option<RevealDisplayMode>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<FontStyle>,
    #[serde(default)]
    pub font_size: Option<u16>,
}

impl Default for RevealVisualConfig {
    fn default() -> Self {
        Self {
            mode: Some(RevealDisplayMode::Shown),
            text: Some(String::from("Press any key or click to continue")),
            color: None,
            font_family: None,
            font_weight: None,
            font_style: None,
            font_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub mode: Option<StatusDisplayMode>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub pending_color: Option<RgbColor>,
    #[serde(default)]
    pub rejected_color: Option<RgbColor>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for StatusVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            mode: Some(StatusDisplayMode::Inline),
            color: None,
            pending_color: Some(RgbColor::rgba(236, 236, 236, 224)),
            rejected_color: Some(RgbColor::rgba(255, 213, 213, 250)),
            position: WidgetPositionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EyeVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
}

impl Default for EyeVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgba(255, 255, 255, 184)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapsLockVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
}

impl Default for CapsLockVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgba(255, 255, 255, 133)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyboardVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_size: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for KeyboardVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            background_color: Some(RgbColor::rgba(255, 255, 255, 10)),
            background_size: Some(46),
            color: Some(RgbColor::rgba(255, 255, 255, 173)),
            size: Some(2),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Right),
                valign: Some(super::VerticalAlign::Top),
                x: Some(-24),
                y: Some(21),
                relative_to: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatteryVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_size: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for BatteryVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            background_color: Some(RgbColor::rgba(255, 255, 255, 10)),
            background_size: Some(46),
            color: Some(RgbColor::rgba(255, 255, 255, 173)),
            size: Some(20),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Right),
                valign: Some(super::VerticalAlign::Top),
                x: Some(-78),
                y: Some(21),
                relative_to: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PowerStatusVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for PowerStatusVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Right),
                valign: Some(super::VerticalAlign::Top),
                x: Some(-24),
                y: Some(17),
                relative_to: None,
            },
        }
    }
}

impl super::VisualConfig {
    pub fn reveal_enabled(&self) -> bool {
        self.reveal
            .as_ref()
            .and_then(|reveal| reveal.mode)
            .unwrap_or(RevealDisplayMode::Shown)
            != RevealDisplayMode::Hidden
    }

    pub fn reveal_mode(&self) -> RevealDisplayMode {
        self.reveal
            .as_ref()
            .and_then(|reveal| reveal.mode)
            .unwrap_or(RevealDisplayMode::Shown)
    }

    pub fn reveal_text(&self) -> String {
        self.reveal
            .as_ref()
            .and_then(|reveal| reveal.text.as_deref())
            .map(|text| super::input::sanitized_reveal_hint(Some(text)))
            .unwrap_or_else(|| self.input_reveal_hint())
    }

    pub fn reveal_color(&self) -> Option<RgbColor> {
        self.reveal.as_ref().and_then(|reveal| reveal.color)
    }

    pub fn reveal_font_family(&self) -> Option<&str> {
        self.reveal
            .as_ref()
            .and_then(|reveal| reveal.font_family.as_deref())
    }

    pub fn reveal_font_weight(&self) -> Option<u16> {
        self.reveal.as_ref().and_then(|reveal| reveal.font_weight)
    }

    pub fn reveal_font_style(&self) -> Option<FontStyle> {
        self.reveal.as_ref().and_then(|reveal| reveal.font_style)
    }

    pub fn reveal_font_size(&self) -> Option<u16> {
        self.reveal.as_ref().and_then(|reveal| reveal.font_size)
    }

    pub fn placeholder_color(&self) -> Option<RgbColor> {
        self.placeholder
            .as_ref()
            .and_then(|placeholder| placeholder.color)
            .or(self.placeholder_color)
    }

    pub fn placeholder_enabled(&self) -> bool {
        self.placeholder
            .as_ref()
            .and_then(|placeholder| placeholder.enabled)
            .unwrap_or(true)
    }

    pub fn status_color(&self) -> Option<RgbColor> {
        self.status
            .as_ref()
            .and_then(|status| status.color)
            .or(self.status_color)
    }

    pub fn status_mode(&self) -> StatusDisplayMode {
        self.status
            .as_ref()
            .and_then(|status| status.mode)
            .unwrap_or(StatusDisplayMode::Inline)
    }

    pub fn status_enabled(&self) -> bool {
        self.status
            .as_ref()
            .and_then(|status| status.enabled)
            .unwrap_or(true)
    }

    pub fn status_pending_color(&self) -> Option<RgbColor> {
        self.status.as_ref().and_then(|status| status.pending_color)
    }

    pub fn status_rejected_color(&self) -> Option<RgbColor> {
        self.status
            .as_ref()
            .and_then(|status| status.rejected_color)
    }

    pub fn status_position(&self) -> WidgetPositionConfig {
        self.status
            .as_ref()
            .map(|status| status.position.clone())
            .unwrap_or_default()
    }

    pub fn eye_icon_color(&self) -> Option<RgbColor> {
        self.eye
            .as_ref()
            .and_then(|eye| eye.color)
            .or(self.eye_icon_color)
    }

    pub fn eye_enabled(&self) -> bool {
        self.eye
            .as_ref()
            .and_then(|eye| eye.enabled)
            .unwrap_or(true)
    }

    pub fn caps_lock_enabled(&self) -> bool {
        self.caps_lock
            .as_ref()
            .and_then(|caps_lock| caps_lock.enabled)
            .unwrap_or(true)
    }

    pub fn caps_lock_color(&self) -> Option<RgbColor> {
        self.caps_lock
            .as_ref()
            .and_then(|caps_lock| caps_lock.color)
    }

    pub fn keyboard_enabled(&self) -> bool {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.enabled)
            .unwrap_or(true)
    }

    pub fn keyboard_color(&self) -> Option<RgbColor> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.color)
            .or(self.keyboard_color)
    }

    pub fn keyboard_background_color(&self) -> Option<RgbColor> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.background_color)
    }

    pub fn keyboard_background_size(&self) -> Option<u16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.background_size)
            .or(self.keyboard_background_size)
    }

    pub fn keyboard_size(&self) -> Option<u16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.size)
            .or(self.keyboard_size)
    }

    pub fn keyboard_position(&self) -> WidgetPositionConfig {
        self.keyboard
            .as_ref()
            .map(|keyboard| keyboard.position.clone())
            .unwrap_or_default()
    }

    pub fn battery_enabled(&self) -> bool {
        self.battery
            .as_ref()
            .and_then(|battery| battery.enabled)
            .unwrap_or(true)
    }

    pub fn battery_color(&self) -> Option<RgbColor> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.color)
            .or(self.battery_color)
    }

    pub fn battery_background_color(&self) -> Option<RgbColor> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.background_color)
            .or(self.battery_background_color)
    }

    pub fn battery_background_size(&self) -> Option<u16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.background_size)
            .or(self.battery_background_size)
    }

    pub fn battery_size(&self) -> Option<u16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.size)
            .or(self.battery_size)
    }

    pub fn battery_position(&self) -> WidgetPositionConfig {
        self.battery
            .as_ref()
            .map(|battery| battery.position.clone())
            .unwrap_or_default()
    }

    pub fn power_status_enabled(&self) -> bool {
        self.power_status
            .as_ref()
            .and_then(|power_status| power_status.enabled)
            .unwrap_or(false)
    }

    pub fn power_status_position(&self) -> WidgetPositionConfig {
        self.power_status
            .as_ref()
            .map(|power_status| power_status.position.clone())
            .unwrap_or_default()
    }
}
