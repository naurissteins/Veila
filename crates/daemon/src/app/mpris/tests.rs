use veila_common::NowPlayingSnapshot;

use super::same_track_snapshot;

#[test]
fn refresh_time_does_not_change_track_identity() {
    let previous = snapshot();
    let current = NowPlayingSnapshot {
        fetched_at_unix: 200,
        ..previous.clone()
    };

    assert!(same_track_snapshot(Some(&previous), Some(&current)));
}

#[test]
fn track_content_changes_are_published() {
    let previous = snapshot();
    for current in [
        NowPlayingSnapshot {
            title: "New title".into(),
            ..previous.clone()
        },
        NowPlayingSnapshot {
            artist: None,
            ..previous.clone()
        },
        NowPlayingSnapshot {
            artwork_path: None,
            ..previous.clone()
        },
    ] {
        assert!(
            !same_track_snapshot(Some(&previous), Some(&current)),
            "{current:?}"
        );
    }
}

#[test]
fn track_presence_changes_are_published() {
    let track = snapshot();

    assert!(same_track_snapshot(None, None));
    assert!(!same_track_snapshot(None, Some(&track)));
    assert!(!same_track_snapshot(Some(&track), None));
}

fn snapshot() -> NowPlayingSnapshot {
    NowPlayingSnapshot {
        title: "Title".into(),
        artist: Some("Artist".into()),
        artwork_path: Some("/tmp/artwork.png".into()),
        fetched_at_unix: 100,
    }
}
