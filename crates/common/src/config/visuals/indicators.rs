use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaceholderVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
}

impl Default for PlaceholderVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(60),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub gap: Option<u16>,
}

impl Default for StatusVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgb(255, 224, 160)),
            opacity: Some(88),
            gap: Some(18),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EyeVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
}

impl Default for EyeVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(72),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapsLockVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
}

impl Default for CapsLockVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
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
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub top_offset: Option<i16>,
    #[serde(default)]
    pub right_offset: Option<i16>,
}

impl Default for KeyboardVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            background_color: Some(RgbColor::rgba(255, 255, 255, 13)),
            background_size: Some(46),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(68),
            size: Some(2),
            top_offset: Some(-24),
            right_offset: Some(8),
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
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub top_offset: Option<i16>,
    #[serde(default)]
    pub right_offset: Option<i16>,
    #[serde(default)]
    pub gap: Option<u16>,
}

impl Default for BatteryVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            background_color: Some(RgbColor::rgba(255, 255, 255, 13)),
            background_size: Some(46),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(68),
            size: Some(20),
            top_offset: Some(-24),
            right_offset: Some(8),
            gap: Some(8),
        }
    }
}

impl super::VisualConfig {
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

    pub fn placeholder_opacity(&self) -> Option<u8> {
        self.placeholder
            .as_ref()
            .and_then(|placeholder| placeholder.opacity)
            .or(self.placeholder_opacity)
    }

    pub fn status_color(&self) -> Option<RgbColor> {
        self.status
            .as_ref()
            .and_then(|status| status.color)
            .or(self.status_color)
    }

    pub fn status_enabled(&self) -> bool {
        self.status
            .as_ref()
            .and_then(|status| status.enabled)
            .unwrap_or(true)
    }

    pub fn status_opacity(&self) -> Option<u8> {
        self.status
            .as_ref()
            .and_then(|status| status.opacity)
            .or(self.status_opacity)
    }

    pub fn status_gap(&self) -> Option<u16> {
        self.status
            .as_ref()
            .and_then(|status| status.gap)
            .or(self.status_gap)
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

    pub fn eye_icon_opacity(&self) -> Option<u8> {
        self.eye
            .as_ref()
            .and_then(|eye| eye.opacity)
            .or(self.eye_icon_opacity)
    }

    pub fn caps_lock_enabled(&self) -> bool {
        self.caps_lock
            .as_ref()
            .and_then(|caps_lock| caps_lock.enabled)
            .unwrap_or(true)
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

    pub fn keyboard_opacity(&self) -> Option<u8> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.opacity)
            .or(self.keyboard_opacity)
    }

    pub fn keyboard_size(&self) -> Option<u16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.size)
            .or(self.keyboard_size)
    }

    pub fn keyboard_top_offset(&self) -> Option<i16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.top_offset)
            .or(self.keyboard_top_offset)
    }

    pub fn keyboard_right_offset(&self) -> Option<i16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.right_offset)
            .or(self.keyboard_right_offset)
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

    pub fn battery_opacity(&self) -> Option<u8> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.opacity)
            .or(self.battery_opacity)
    }

    pub fn battery_size(&self) -> Option<u16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.size)
            .or(self.battery_size)
    }

    pub fn battery_top_offset(&self) -> Option<i16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.top_offset)
            .or(self.battery_top_offset)
    }

    pub fn battery_right_offset(&self) -> Option<i16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.right_offset)
            .or(self.battery_right_offset)
    }

    pub fn battery_gap(&self) -> Option<u16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.gap)
            .or(self.battery_gap)
    }
}
