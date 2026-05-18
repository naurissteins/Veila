use serde::{Deserialize, Serialize};

use super::{HorizontalAlign, RgbColor, VerticalAlign, WidgetPositionConfig};
use crate::PowerAction;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PowerVisualConfig {
    #[serde(default)]
    pub suspend: Option<PowerButtonVisualConfig>,
    #[serde(default)]
    pub reboot: Option<PowerButtonVisualConfig>,
    #[serde(default)]
    pub poweroff: Option<PowerButtonVisualConfig>,
}

impl Default for PowerVisualConfig {
    fn default() -> Self {
        Self {
            suspend: Some(PowerButtonVisualConfig::for_action(PowerAction::Suspend)),
            reboot: Some(PowerButtonVisualConfig::for_action(PowerAction::Reboot)),
            poweroff: Some(PowerButtonVisualConfig::for_action(PowerAction::Poweroff)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PowerButtonVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_size: Option<u16>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub confirm: Option<bool>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl PowerButtonVisualConfig {
    fn for_action(action: PowerAction) -> Self {
        let (x, confirm) = match action {
            PowerAction::Suspend => (-132, false),
            PowerAction::Reboot => (-186, true),
            PowerAction::Poweroff => (-240, true),
        };

        Self {
            enabled: Some(false),
            background_color: Some(RgbColor::rgba(255, 255, 255, 10)),
            background_size: Some(46),
            radius: Some(23),
            color: Some(RgbColor::rgba(255, 255, 255, 173)),
            size: Some(20),
            confirm: Some(confirm),
            position: WidgetPositionConfig {
                halign: Some(HorizontalAlign::Right),
                valign: Some(VerticalAlign::Top),
                x: Some(x),
                y: Some(21),
                relative_to: None,
            },
        }
    }
}

impl super::VisualConfig {
    fn power_button(&self, action: PowerAction) -> PowerButtonVisualConfig {
        let configured = self.power.as_ref().and_then(|power| match action {
            PowerAction::Suspend => power.suspend.clone(),
            PowerAction::Reboot => power.reboot.clone(),
            PowerAction::Poweroff => power.poweroff.clone(),
        });

        configured.unwrap_or_else(|| PowerButtonVisualConfig::for_action(action))
    }

    pub fn power_button_enabled(&self, action: PowerAction) -> bool {
        self.power_button(action).enabled.unwrap_or(false)
    }

    pub fn power_button_background_color(&self, action: PowerAction) -> Option<RgbColor> {
        self.power_button(action).background_color
    }

    pub fn power_button_background_size(&self, action: PowerAction) -> Option<u16> {
        self.power_button(action).background_size
    }

    pub fn power_button_radius(&self, action: PowerAction) -> Option<u16> {
        self.power_button(action).radius
    }

    pub fn power_button_color(&self, action: PowerAction) -> Option<RgbColor> {
        self.power_button(action).color
    }

    pub fn power_button_size(&self, action: PowerAction) -> Option<u16> {
        self.power_button(action).size
    }

    pub fn power_button_confirm(&self, action: PowerAction) -> bool {
        self.power_button(action)
            .confirm
            .unwrap_or(!matches!(action, PowerAction::Suspend))
    }

    pub fn power_button_position(&self, action: PowerAction) -> WidgetPositionConfig {
        self.power_button(action).position
    }
}
