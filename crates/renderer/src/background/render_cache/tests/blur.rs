use std::{fs, path::PathBuf};

use super::super::{CacheSource, cache_path, cache_root, cache_source_key, write_buffer};
use super::{load_cached_buffer_at, store_cached_buffer_at};
use crate::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{BackgroundGradient, BackgroundTreatment, GeneratedBackground},
    cache::stable_hash,
};

#[test]
fn capped_wallpaper_entries_miss_for_stronger_blur_and_all_variants() {
    let root = fixture("wallpaper");
    let wallpaper = root.join("wallpaper");
    let source = CacheSource::Path {
        path: &wallpaper,
        fallback: ClearColor::opaque(0, 0, 0),
    };
    let size = FrameSize::new(2, 1);
    let old = SoftwareBuffer::solid(size, ClearColor::opaque(1, 2, 3)).unwrap();
    let new = SoftwareBuffer::solid(size, ClearColor::opaque(4, 5, 6)).unwrap();
    for strength in [0, 1, 6, 12, 13, 24, 25, 64, 128, 255] {
        let treatment = BackgroundTreatment {
            blur_radius: strength,
            ..BackgroundTreatment::default()
        };
        for variant in [None, Some("backdrop:v2"), Some("scene:v1")] {
            let old_path = legacy_path(source, size, treatment, variant, &root);
            write_buffer(&old_path, size, &old).unwrap();
            let loaded = load_cached_buffer_at(source, size, treatment, variant, &root).unwrap();
            assert_eq!(loaded, (strength <= 12).then(|| old.clone()));
            store_cached_buffer_at(source, size, treatment, &new, variant, &root).unwrap();
            assert_eq!(
                load_cached_buffer_at(source, size, treatment, variant, &root).unwrap(),
                Some(new.clone())
            );
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn generated_background_keys_keep_existing_entries_at_all_strengths() {
    let root = fixture("generated");
    let source = CacheSource::Generated(GeneratedBackground::Gradient(BackgroundGradient {
        top_left: ClearColor::opaque(0, 0, 0),
        top_right: ClearColor::opaque(255, 0, 0),
        bottom_left: ClearColor::opaque(0, 255, 0),
        bottom_right: ClearColor::opaque(0, 0, 255),
    }));
    let size = FrameSize::new(2, 1);
    for strength in [0, 12, 13, 64, 255] {
        let treatment = BackgroundTreatment {
            blur_radius: strength,
            ..BackgroundTreatment::default()
        };
        for variant in [None, Some("backdrop:v2"), Some("scene:v1")] {
            assert_eq!(
                cache_path(source, size, treatment, variant, Some(&root)).unwrap(),
                legacy_path(source, size, treatment, variant, &root)
            );
        }
    }
    fs::remove_dir_all(root).unwrap();
}

fn legacy_path(
    source: CacheSource<'_>,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: Option<&str>,
    root: &std::path::Path,
) -> PathBuf {
    let key = stable_hash(&format!(
        "renderer:{}:{}",
        env!("CARGO_PKG_VERSION"),
        cache_source_key(source, size).unwrap()
    ));
    let key = stable_hash(&format!(
        "{key}:{:?}:{:?}:{:?}:{:?}",
        treatment.blur_radius, treatment.dim_strength, treatment.tint, treatment.scaling
    ));
    let key = stable_hash(&format!("{key}:{}", variant.unwrap_or_default()));
    cache_root(Some(root))
        .unwrap()
        .join(format!("{key:016x}.argb"))
}

fn fixture(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("veila-strong-blur-{name}-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("wallpaper"), b"stub").unwrap();
    root
}
