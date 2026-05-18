use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PowerAction {
    #[serde(rename = "suspend")]
    Suspend,
    #[serde(rename = "reboot")]
    Reboot,
    #[serde(rename = "poweroff")]
    Poweroff,
}
