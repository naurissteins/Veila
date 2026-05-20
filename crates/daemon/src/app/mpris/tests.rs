use super::{PlayerDescriptor, normalize_filter_value, player_is_excluded, player_is_included};

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
