use super::{load_cached_buffer_at, store_cached_buffer_at};
use crate::{ClearColor, FrameSize, SoftwareBuffer, background::BackgroundTreatment};
use std::{fs, path::PathBuf};

use super::super::{CacheSource, cache_path};

#[test]
fn fallback_channels_separate_base_and_variant_entries() {
    let root = fixture("channels");
    let wallpaper = root.join("wallpaper");
    let size = FrameSize::new(2, 1);
    let treatment = BackgroundTreatment::default();
    for variant in [None, Some("backdrop:v1"), Some("scene:v1")] {
        let original = cache_path(
            CacheSource::Path {
                path: &wallpaper,
                fallback: ClearColor::rgba(1, 2, 3, 4),
            },
            size,
            treatment,
            variant,
            Some(&root),
        )
        .unwrap();
        for changed in [(5, 2, 3, 4), (1, 5, 3, 4), (1, 2, 5, 4), (1, 2, 3, 5)] {
            let fallback = ClearColor::rgba(changed.0, changed.1, changed.2, changed.3);
            let key = cache_path(
                CacheSource::Path {
                    path: &wallpaper,
                    fallback,
                },
                size,
                treatment,
                variant,
                Some(&root),
            )
            .unwrap();
            assert_ne!(original, key);
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_file_entries_are_misses_for_base_and_variants() {
    let root = fixture("legacy");
    let wallpaper = root.join("wallpaper");
    let size = FrameSize::new(2, 1);
    let treatment = BackgroundTreatment::default();
    let buffer = SoftwareBuffer::solid(size, ClearColor::opaque(0, 0, 0)).unwrap();
    for variant in [None, Some("backdrop:v1"), Some("scene:v1")] {
        let source = format!(
            "image:v2:{}:2x1",
            crate::cache::file_cache_key(&wallpaper).unwrap()
        );
        let key =
            crate::cache::stable_hash(&format!("renderer:{}:{source}", env!("CARGO_PKG_VERSION")));
        let key = crate::cache::stable_hash(&format!(
            "{key}:{:?}:{:?}:{:?}:{:?}",
            treatment.blur_radius, treatment.dim_strength, treatment.tint, treatment.scaling
        ));
        let key = crate::cache::stable_hash(&format!("{key}:{}", variant.unwrap_or_default()));
        let legacy = super::super::cache_root(Some(&root))
            .unwrap()
            .join(format!("{key:016x}.argb"));
        super::super::write_buffer(&legacy, size, &buffer).unwrap();
        let loaded = load_cached_buffer_at(
            CacheSource::Path {
                path: &wallpaper,
                fallback: ClearColor::opaque(0, 0, 0),
            },
            size,
            treatment,
            variant,
            &root,
        )
        .unwrap();
        assert!(
            loaded.is_none(),
            "even black fallback must invalidate v2 entries"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupt_entries_miss_and_can_be_repaired_without_touching_other_colours() {
    let root = fixture("corrupt");
    let wallpaper = root.join("wallpaper");
    let size = FrameSize::new(2, 1);
    let treatment = BackgroundTreatment::default();
    let red = ClearColor::opaque(255, 0, 0);
    let black = ClearColor::opaque(0, 0, 0);
    let red_source = CacheSource::Path {
        path: &wallpaper,
        fallback: red,
    };
    let black_source = CacheSource::Path {
        path: &wallpaper,
        fallback: black,
    };
    let red_buffer = SoftwareBuffer::solid(size, red).unwrap();
    let black_buffer = SoftwareBuffer::solid(size, black).unwrap();
    for variant in [None, Some("backdrop:v1"), Some("scene:v1")] {
        store_cached_buffer_at(black_source, size, treatment, &black_buffer, variant, &root)
            .unwrap();
        store_cached_buffer_at(red_source, size, treatment, &red_buffer, variant, &root).unwrap();
        let path = cache_path(red_source, size, treatment, variant, Some(&root)).unwrap();
        fs::write(&path, b"truncated").unwrap();
        assert!(
            load_cached_buffer_at(red_source, size, treatment, variant, &root)
                .unwrap()
                .is_none()
        );
        store_cached_buffer_at(red_source, size, treatment, &red_buffer, variant, &root).unwrap();
        assert_eq!(
            load_cached_buffer_at(red_source, size, treatment, variant, &root).unwrap(),
            Some(red_buffer.clone())
        );
        assert_eq!(
            load_cached_buffer_at(black_source, size, treatment, variant, &root).unwrap(),
            Some(black_buffer.clone())
        );
    }
    fs::remove_dir_all(root).unwrap();
}

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("veila-fallback-{name}-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("wallpaper"), b"stub").unwrap();
    root
}
