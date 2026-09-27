use std::{fs, path::Path, process::Command};

use veila_renderer::{
    ClearColor, FrameSize,
    avatar::AvatarAsset,
    background::{
        BackgroundAsset, BackgroundTreatment, SourceCacheStatus, load_cached_render,
        prewarm_rendered, prewarm_source,
    },
    cache::{CacheKind, CachePrunePolicy, prune_cache},
};

#[test]
fn corrupt_disk_caches_recover_from_original_images() {
    if let Some(root) = std::env::var_os("VEILA_TEST_CACHE_RECOVERY") {
        check_recovery(Path::new(&root));
        return;
    }
    isolated_run(
        "corrupt_disk_caches_recover_from_original_images",
        "recovery",
        false,
    );
}

#[test]
fn unavailable_cache_directory_does_not_block_image_rendering() {
    if let Some(root) = std::env::var_os("VEILA_TEST_CACHE_RECOVERY") {
        let path = create_image(Path::new(&root));
        assert!(
            BackgroundAsset::load(
                Some(&path),
                ClearColor::opaque(0, 0, 0),
                None,
                BackgroundTreatment::default()
            )
            .unwrap()
            .render(FrameSize::new(16, 9))
            .is_ok()
        );
        assert!(AvatarAsset::load(&path).is_ok());
        return;
    }
    isolated_run(
        "unavailable_cache_directory_does_not_block_image_rendering",
        "unavailable",
        true,
    );
}

fn isolated_run(test: &str, suffix: &str, block_cache: bool) {
    let root = std::env::temp_dir().join(format!("veila-cache-{suffix}-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let cache = root.join("cache");
    if block_cache {
        fs::write(&cache, b"not a directory").unwrap();
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("VEILA_TEST_CACHE_RECOVERY", &root)
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_image(root: &Path) -> std::path::PathBuf {
    let path = root.join("wallpaper.png");
    image::RgbaImage::from_fn(8, 4, |x, y| {
        image::Rgba([x as u8 * 25, y as u8 * 55, 99, 255])
    })
    .save(&path)
    .unwrap();
    path
}

fn check_recovery(root: &Path) {
    let path = create_image(root);
    let original = fs::read(&path).unwrap();
    let size = FrameSize::new(16, 9);
    let treatment = BackgroundTreatment::default();
    let fallback = ClearColor::opaque(0, 0, 0);
    let expected = BackgroundAsset::load(Some(&path), fallback, None, treatment)
        .unwrap()
        .render(size)
        .unwrap();
    let avatar_key = AvatarAsset::load(&path).unwrap().cache_key();
    for round in 0..3 {
        assert_eq!(
            prewarm_rendered(&path, fallback, treatment, &[size])
                .unwrap()
                .warmed_sizes,
            1
        );
        assert_eq!(
            load_cached_render(&path, size, treatment).unwrap().unwrap(),
            expected
        );
        assert_eq!(AvatarAsset::load(&path).unwrap().cache_key(), avatar_key);
        assert_eq!(prewarm_source(&path).unwrap(), SourceCacheStatus::Hit);
        assert_eq!(
            prewarm_rendered(&path, fallback, treatment, &[size])
                .unwrap()
                .warmed_sizes,
            0
        );
        for kind in [
            CacheKind::RenderedBackground,
            CacheKind::SourceImage,
            CacheKind::Avatar,
        ] {
            for entry in fs::read_dir(root.join("cache/veila").join(kind.directory())).unwrap() {
                let path = entry.unwrap().path();
                let mut bytes = fs::read(&path).unwrap();
                match round {
                    0 => bytes.truncate(5),
                    1 => bytes.truncate(16),
                    _ => bytes.push(0),
                }
                fs::write(path, bytes).unwrap();
            }
        }
        assert!(
            load_cached_render(&path, size, treatment)
                .unwrap()
                .is_none()
        );
        assert!(AvatarAsset::load_cached(&path).unwrap().is_none());
    }
    for kind in [
        CacheKind::RenderedBackground,
        CacheKind::SourceImage,
        CacheKind::Avatar,
    ] {
        let report = prune_cache(
            kind,
            CachePrunePolicy {
                max_bytes: 0,
                max_age: std::time::Duration::ZERO,
            },
        )
        .unwrap();
        assert!(report.removed_files > 0);
        assert_eq!(report.retained_bytes, 0);
    }
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(
        BackgroundAsset::load(Some(&path), fallback, None, treatment)
            .unwrap()
            .render(size)
            .unwrap(),
        expected
    );
}
