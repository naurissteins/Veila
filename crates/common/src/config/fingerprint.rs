use serde::{Deserialize, Deserializer, Serialize, de};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(
        default = "default_max_failed_attempts",
        deserialize_with = "deserialize_max_failed_attempts"
    )]
    pub max_failed_attempts: u8,
}

impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_failed_attempts: default_max_failed_attempts(),
        }
    }
}

const fn default_max_failed_attempts() -> u8 {
    5
}

fn deserialize_max_failed_attempts<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let value = u8::deserialize(deserializer)?;
    if value == 0 {
        return Err(de::Error::custom("max_failed_attempts must be at least 1"));
    }
    Ok(value)
}
