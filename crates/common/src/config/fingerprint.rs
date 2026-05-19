use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FingerprintConfig {
    #[serde(default)]
    pub enabled: bool,
}
