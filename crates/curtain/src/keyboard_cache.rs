use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

pub(crate) fn load_keyboard_layout_label() -> Option<String> {
    let path = cache_path().ok()?;
    let raw = fs::read_to_string(path).ok()?;
    normalize_label(&raw)
}

pub(crate) fn store_keyboard_layout_label(label: &str) {
    let Some(label) = normalize_label(label) else {
        return;
    };

    if let Err(error) = store_keyboard_layout_label_inner(&label) {
        tracing::debug!("failed to store keyboard layout label cache: {error:#}");
    }
}

fn store_keyboard_layout_label_inner(label: &str) -> Result<()> {
    let path = cache_path()?;
    let parent = path
        .parent()
        .context("keyboard layout cache path has no parent")?;
    fs::create_dir_all(parent).context("failed to create keyboard layout cache directory")?;
    fs::write(path, label).context("failed to write keyboard layout cache")?;
    Ok(())
}

fn normalize_label(label: &str) -> Option<String> {
    let label = label.trim();
    if label.is_empty() || label.len() > 8 {
        return None;
    }

    label
        .chars()
        .all(|character| character.is_ascii_alphanumeric())
        .then(|| label.to_ascii_uppercase())
}

fn cache_path() -> Result<PathBuf> {
    Ok(cache_root()?.join("keyboard-layout.txt"))
}

fn cache_root() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .context("failed to resolve XDG cache directory")?;

    Ok(base.join("veila"))
}

#[cfg(test)]
mod tests {
    use super::normalize_label;

    #[test]
    fn normalizes_cached_keyboard_labels() {
        assert_eq!(normalize_label("lv"), Some(String::from("LV")));
        assert_eq!(normalize_label(" EN "), Some(String::from("EN")));
        assert_eq!(normalize_label(""), None);
        assert_eq!(normalize_label("too-long-label"), None);
    }
}
