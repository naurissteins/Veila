use anyhow::anyhow;

use super::{
    MAX_ARTWORK_URL_BYTES, MAX_METADATA_CHARS, PlayerDescriptor, normalize_artwork_url,
    normalize_filter_value, normalize_string, optional_property_string, player_is_excluded,
    player_is_included,
};

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
fn excludes_players_by_identity_case_insensitively() {
    let player = PlayerDescriptor {
        bus_name: String::from("org.mpris.MediaPlayer2.firefox"),
        identity: Some(String::from("Firefox")),
        desktop_entry: Some(String::from("firefox")),
    };

    assert!(player_is_excluded(&player, &[String::from("firefox")]));
}

#[test]
fn includes_all_players_when_include_list_is_empty() {
    let player = PlayerDescriptor {
        bus_name: String::from("org.mpris.MediaPlayer2.firefox"),
        identity: Some(String::from("Firefox")),
        desktop_entry: Some(String::from("firefox")),
    };

    assert!(player_is_included(&player, &[]));
}

#[test]
fn includes_matching_players_by_identity_case_insensitively() {
    let player = PlayerDescriptor {
        bus_name: String::from("org.mpris.MediaPlayer2.spotify"),
        identity: Some(String::from("Spotify")),
        desktop_entry: Some(String::from("spotify")),
    };

    assert!(player_is_included(&player, &[String::from("spotify")]));
    assert!(!player_is_included(&player, &[String::from("firefox")]));
}

#[test]
fn excludes_players_by_bus_name_base_for_instance_suffixes() {
    let player = PlayerDescriptor {
        bus_name: String::from("org.mpris.MediaPlayer2.chromium.instance458"),
        identity: None,
        desktop_entry: None,
    };

    assert!(player_is_excluded(&player, &[String::from("Chromium")]));
}

#[test]
fn ignores_empty_filter_entries() {
    let player = PlayerDescriptor {
        bus_name: String::from("org.mpris.MediaPlayer2.spotify"),
        identity: Some(String::from("Spotify")),
        desktop_entry: Some(String::from("spotify")),
    };

    assert!(!player_is_excluded(
        &player,
        &[String::from(" "), String::from("")],
    ));
    assert!(!player_is_included(
        &player,
        &[String::from(" "), String::from("")],
    ));
    assert_eq!(normalize_filter_value(" Firefox "), "firefox");
}

#[test]
fn exclude_filters_override_include_filters() {
    let player = PlayerDescriptor {
        bus_name: String::from("org.mpris.MediaPlayer2.firefox"),
        identity: Some(String::from("Firefox")),
        desktop_entry: Some(String::from("firefox")),
    };

    assert!(player_is_included(&player, &[String::from("Firefox")]));
    assert!(player_is_excluded(&player, &[String::from("Firefox")]));
}

#[test]
fn optional_property_errors_become_missing_values() {
    let value = optional_property_string(
        Err(anyhow!("error occurred in Get")),
        "org.mpris.MediaPlayer2.chromium.instance458",
        "DesktopEntry",
    );

    assert_eq!(value, None);
    assert_eq!(
        optional_property_string(
            Ok(Some(String::from("chromium"))),
            "org.mpris.MediaPlayer2.chromium.instance458",
            "DesktopEntry",
        ),
        Some(String::from("chromium"))
    );
}
