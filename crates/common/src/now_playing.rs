use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NowPlayingSnapshot {
    pub title: String,
    pub artist: Option<String>,
    pub artwork_path: Option<PathBuf>,
    pub fetched_at_unix: i64,
}
