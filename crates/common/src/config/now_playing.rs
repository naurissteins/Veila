use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NowPlayingConfig {
    #[serde(default)]
    pub include_players: Vec<String>,
    #[serde(default)]
    pub exclude_players: Vec<String>,
}
