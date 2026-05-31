use std::path::PathBuf;

use veila_common::NowPlayingSnapshot;
use veila_renderer::cover::CoverArtAsset;

#[derive(Debug, Clone)]
pub(super) struct NowPlayingWidgetData {
    pub(super) title: String,
    pub(super) artist: Option<String>,
    pub(super) artwork_path: Option<PathBuf>,
    pub(super) artwork: Option<CoverArtAsset>,
}

pub(super) fn widget_data(snapshot: Option<NowPlayingSnapshot>) -> Option<NowPlayingWidgetData> {
    let snapshot = snapshot?;
    let title = normalize(snapshot.title)?;
    let artist = snapshot.artist.and_then(normalize);
    let artwork_path = snapshot.artwork_path;
    let artwork = artwork_path.as_deref().and_then(|path| {
        CoverArtAsset::load(path)
            .map_err(|error| {
                tracing::debug!(
                    path = %path.display(),
                    "failed to load now playing artwork: {error:#}"
                );
                error
            })
            .ok()
    });

    Some(NowPlayingWidgetData {
        title,
        artist,
        artwork_path,
        artwork,
    })
}

pub(super) fn same_widget_data(
    left: Option<&NowPlayingWidgetData>,
    right: Option<&NowPlayingWidgetData>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.title == right.title
                && left.artist == right.artist
                && left.artwork_path == right.artwork_path
                && left.artwork.is_some() == right.artwork.is_some()
        }
        _ => false,
    }
}

fn normalize(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use veila_common::NowPlayingSnapshot;

    use super::{same_widget_data, widget_data};

    const ONE_PIXEL_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[test]
    fn hides_widget_without_title() {
        let widget = widget_data(Some(NowPlayingSnapshot {
            title: String::from("   "),
            artist: Some(String::from("Artist")),
            artwork_path: None,
            fetched_at_unix: 0,
        }));

        assert!(widget.is_none());
    }

    #[test]
    fn keeps_title_and_artist() {
        let widget = widget_data(Some(NowPlayingSnapshot {
            title: String::from(" Track "),
            artist: Some(String::from(" Artist ")),
            artwork_path: None,
            fetched_at_unix: 0,
        }))
        .expect("widget");

        assert_eq!(widget.title, "Track");
        assert_eq!(widget.artist.as_deref(), Some("Artist"));
    }

    #[test]
    fn compares_widget_identity_without_loaded_artwork_pixels() {
        let left = widget_data(Some(NowPlayingSnapshot {
            title: String::from("Track"),
            artist: Some(String::from("Artist")),
            artwork_path: Some(PathBuf::from("/tmp/art.png")),
            fetched_at_unix: 0,
        }));
        let right = widget_data(Some(NowPlayingSnapshot {
            title: String::from("Track"),
            artist: Some(String::from("Artist")),
            artwork_path: Some(PathBuf::from("/tmp/art.png")),
            fetched_at_unix: 10,
        }));

        assert!(same_widget_data(left.as_ref(), right.as_ref()));
    }

    #[test]
    fn detects_late_artwork_load_for_same_track_and_path() {
        let path = std::env::temp_dir().join(format!(
            "veila-now-playing-artwork-{}.png",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        let before = widget_data(Some(NowPlayingSnapshot {
            title: String::from("Track"),
            artist: Some(String::from("Artist")),
            artwork_path: Some(path.clone()),
            fetched_at_unix: 0,
        }))
        .expect("widget before artwork exists");

        fs::write(&path, ONE_PIXEL_PNG).expect("write artwork");
        let after = widget_data(Some(NowPlayingSnapshot {
            title: String::from("Track"),
            artist: Some(String::from("Artist")),
            artwork_path: Some(path.clone()),
            fetched_at_unix: 10,
        }))
        .expect("widget after artwork exists");

        let _ = fs::remove_file(path);

        assert!(before.artwork.is_none());
        assert!(after.artwork.is_some());
        assert!(!same_widget_data(Some(&before), Some(&after)));
    }
}
