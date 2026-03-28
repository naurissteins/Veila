use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatterySnapshot {
    pub percent: u8,
    pub charging: bool,
}
