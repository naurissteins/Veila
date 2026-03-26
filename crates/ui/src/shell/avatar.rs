use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use veila_renderer::avatar::AvatarAsset;

use super::ShellState;

pub(super) fn load_avatar(avatar_path: Option<PathBuf>) -> AvatarAsset {
    for path in avatar_candidates(avatar_path) {
        match AvatarAsset::load(&path) {
            Ok(avatar) => return avatar,
            Err(error) => {
                tracing::warn!(path = %path.display(), "failed to load avatar image: {error}")
            }
        }
    }

    AvatarAsset::placeholder()
}

pub(super) fn current_retry_seconds(retry_until: Instant) -> Option<u64> {
    let seconds = retry_until
        .saturating_duration_since(Instant::now())
        .as_millis()
        .div_ceil(1_000) as u64;

    if seconds == 0 { None } else { Some(seconds) }
}

pub(super) fn username_text(
    show_username: bool,
    username_override: Option<String>,
) -> Option<String> {
    if !show_username {
        return None;
    }

    username_override
        .map(|username| username.trim().to_string())
        .filter(|username| !username.is_empty())
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("LOGNAME").ok())
        .map(|username| username.trim().to_string())
        .filter(|username| !username.is_empty())
}

fn avatar_candidates(explicit: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = explicit {
        candidates.push(path);
    }

    if let Some(face_path) = default_face_path()
        && !candidates.iter().any(|path| path == &face_path)
    {
        candidates.push(face_path);
    }

    candidates
}

fn default_face_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = Path::new(&home).join(".face");
    path.is_file().then_some(path)
}

impl ShellState {
    pub(super) fn bump_static_scene_revision(&mut self) {
        self.static_scene_revision = self.static_scene_revision.saturating_add(1);
    }
}
