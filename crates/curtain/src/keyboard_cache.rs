use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::mpsc::{Sender, channel},
    thread,
};

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

pub(crate) fn start_keyboard_layout_writer() -> Option<Sender<String>> {
    let (sender, receiver) = channel::<String>();
    match thread::Builder::new()
        .name(String::from("veila-keyboard-cache"))
        .spawn(move || {
            while let Ok(mut label) = receiver.recv() {
                while let Ok(latest) = receiver.try_recv() {
                    label = latest;
                }
                store_keyboard_layout_label(&label);
            }
        }) {
        Ok(_) => Some(sender),
        Err(error) => {
            tracing::warn!(%error, "failed to start keyboard layout cache writer");
            None
        }
    }
}

fn store_keyboard_layout_label_inner(label: &str) -> Result<()> {
    let path = cache_path()?;
    write_private_label(&path, label)
}

fn write_private_label(path: &Path, label: &str) -> Result<()> {
    let parent = path
        .parent()
        .context("keyboard layout cache path has no parent")?;
    fs::create_dir_all(parent).context("failed to create keyboard layout cache directory")?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .context("failed to secure keyboard layout cache directory")?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .context("failed to open keyboard layout cache")?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .context("failed to secure keyboard layout cache")?;
    file.set_len(0)
        .context("failed to truncate keyboard layout cache")?;
    file.write_all(label.as_bytes())
        .context("failed to write keyboard layout cache")?;
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
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
    };

    use super::{normalize_label, write_private_label};

    #[test]
    fn normalizes_cached_keyboard_labels() {
        assert_eq!(normalize_label("lv"), Some(String::from("LV")));
        assert_eq!(normalize_label(" EN "), Some(String::from("EN")));
        assert_eq!(normalize_label(""), None);
        assert_eq!(normalize_label("too-long-label"), None);
    }

    #[test]
    fn cached_label_uses_private_permissions() {
        let root = std::env::temp_dir().join(format!("veila-keyboard-mode-{}", std::process::id()));
        fs::create_dir_all(&root).expect("cache root");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).expect("old directory mode");
        let path = root.join("keyboard-layout.txt");
        fs::write(&path, "OLD").expect("old label");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("old file mode");

        write_private_label(&path, "EN").expect("write label");

        assert_eq!(
            fs::metadata(&root).expect("directory metadata").mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&path).expect("file metadata").mode() & 0o777,
            0o600
        );
        assert_eq!(fs::read_to_string(&path).expect("label"), "EN");
        fs::remove_dir_all(root).expect("cleanup");
    }
}
