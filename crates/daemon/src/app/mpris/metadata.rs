use std::{collections::HashMap, path::PathBuf};

use zbus::zvariant::OwnedValue;

#[cfg(test)]
mod tests;

const MAX_METADATA_CHARS: usize = 256;
const MAX_ARTWORK_URL_BYTES: usize = 4096;

pub(super) fn metadata_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = metadata.get(key)?.clone();
    normalize_string(String::try_from(value).ok()?)
}

pub(super) fn metadata_artwork_url(metadata: &HashMap<String, OwnedValue>) -> Option<String> {
    let value = metadata.get("mpris:artUrl")?.clone();
    // artwork paths use a byte bound and must never be truncated into a different path
    normalize_artwork_url(String::try_from(value).ok()?)
}

fn normalize_artwork_url(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty() && trimmed.len() <= MAX_ARTWORK_URL_BYTES).then(|| trimmed.to_owned())
}

pub(super) fn metadata_string_list_first(
    metadata: &HashMap<String, OwnedValue>,
    key: &str,
) -> Option<String> {
    let value = metadata.get(key)?.clone();
    let values = Vec::<String>::try_from(value).ok()?;
    values.into_iter().find_map(normalize_string)
}

pub(super) fn normalize_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(MAX_METADATA_CHARS).collect())
}

pub(super) fn resolve_artwork_path(value: String) -> Option<PathBuf> {
    if let Some(path) = value.strip_prefix("file://localhost") {
        return normalize_path(path);
    }

    if let Some(path) = value.strip_prefix("file://") {
        return normalize_path(path);
    }

    normalize_path(&value)
}

fn normalize_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') {
        return None;
    }

    let path = PathBuf::from(trimmed);
    path.is_file().then_some(path)
}
