use serde::{Deserialize, Serialize};

use crate::battery::BatterySnapshot;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatteryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_refresh_seconds")]
    pub refresh_seconds: u16,
    #[serde(default)]
    pub mock_percent: Option<u8>,
    #[serde(default)]
    pub mock_charging: Option<bool>,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            refresh_seconds: default_refresh_seconds(),
            mock_percent: None,
            mock_charging: None,
        }
    }
}

impl BatteryConfig {
    pub fn mock_snapshot(&self) -> Option<BatterySnapshot> {
        self.mock_percent.map(|percent| BatterySnapshot {
            percent: percent.min(100),
            charging: self.mock_charging.unwrap_or(false),
        })
    }
}

const fn default_refresh_seconds() -> u16 {
    30
}
