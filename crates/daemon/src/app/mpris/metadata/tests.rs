use super::{MAX_ARTWORK_URL_BYTES, MAX_METADATA_CHARS, normalize_artwork_url, normalize_string};

#[test]
fn normalizes_and_caps_long_unicode_metadata() {
    let value = format!("  {}  ", "🎵".repeat(MAX_METADATA_CHARS + 20));
    let normalized = normalize_string(value).expect("nonempty metadata");
    assert_eq!(normalized, "🎵".repeat(MAX_METADATA_CHARS));
}

#[test]
fn rejects_oversized_artwork_url_without_truncating_path() {
    assert_eq!(
        normalize_artwork_url("/".repeat(MAX_ARTWORK_URL_BYTES + 1)),
        None
    );
}

#[test]
fn missing_and_mistyped_metadata_is_absent() {
    let metadata = std::collections::HashMap::from([
        (
            "xesam:title".into(),
            zbus::zvariant::OwnedValue::from(42_i64),
        ),
        (
            "mpris:artUrl".into(),
            zbus::zvariant::OwnedValue::from(true),
        ),
    ]);

    assert_eq!(super::metadata_string(&metadata, "xesam:title"), None);
    assert_eq!(super::metadata_string(&metadata, "missing"), None);
    assert_eq!(super::metadata_artwork_url(&metadata), None);
}

#[test]
fn artist_list_uses_first_nonempty_normalized_entry() {
    let artists = zbus::zvariant::Value::from(vec![" ", "  Artist 🎵  ", "Other"]);
    let metadata = std::collections::HashMap::from([(
        "xesam:artist".into(),
        zbus::zvariant::OwnedValue::try_from(artists).expect("owned artists"),
    )]);

    assert_eq!(
        super::metadata_string_list_first(&metadata, "xesam:artist"),
        Some("Artist 🎵".into())
    );
}

#[test]
fn scalar_artist_metadata_is_rejected() {
    let metadata = std::collections::HashMap::from([(
        "xesam:artist".into(),
        zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from("Artist"))
            .expect("owned string"),
    )]);

    assert_eq!(
        super::metadata_string_list_first(&metadata, "xesam:artist"),
        None
    );
}

#[test]
fn artwork_url_at_byte_limit_is_kept_whole() {
    let value = format!("/{}", "é".repeat((MAX_ARTWORK_URL_BYTES - 2) / 2)) + "x";

    assert_eq!(normalize_artwork_url(format!("  {value}  ")), Some(value));
}

#[test]
fn artwork_url_limit_counts_unicode_bytes() {
    let value = "é".repeat(MAX_ARTWORK_URL_BYTES / 2 + 1);

    assert_eq!(normalize_artwork_url(value), None);
}

#[test]
fn artwork_paths_preserve_literal_file_names() {
    let directory = ArtworkDirectory::new();
    let file = directory.0.join("cover %20 🎵.png");
    std::fs::write(&file, b"art").expect("artwork file");

    for value in [
        file.display().to_string(),
        format!("file://{}", file.display()),
        format!("file://localhost{}", file.display()),
    ] {
        assert_eq!(super::resolve_artwork_path(value), Some(file.clone()));
    }
}

#[test]
fn artwork_paths_reject_nonfiles_and_remote_urls() {
    let directory = ArtworkDirectory::new();
    for value in [
        directory.0.display().to_string(),
        directory.0.join("missing.png").display().to_string(),
        "relative.png".into(),
        "https://example.invalid/cover.png".into(),
        "file://remote/cover.png".into(),
        String::new(),
    ] {
        assert_eq!(super::resolve_artwork_path(value.clone()), None, "{value}");
    }
}

#[test]
fn blank_metadata_is_absent() {
    assert_eq!(normalize_string(" \n\t ".into()), None);
    assert_eq!(normalize_artwork_url(" \n\t ".into()), None);
}

struct ArtworkDirectory(std::path::PathBuf);

impl ArtworkDirectory {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "veila-mpris-artwork-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).expect("artwork directory");
        Self(directory)
    }
}

impl Drop for ArtworkDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
