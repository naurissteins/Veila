use super::{CacheKind, CachePrunePolicy, prune_cache_at};
use std::{
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn prunes_oldest_render_cache_entries_to_size_limit() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-render-prune-size-test-{unique}"));
    let cache_dir = super::super::root(Some(&root), "backgrounds").expect("cache root");
    fs::create_dir_all(&cache_dir).expect("cache dir");

    let old = cache_dir.join("old.argb");
    let new = cache_dir.join("new.argb");
    fs::write(&old, [1u8; 10]).expect("old file");
    std::thread::sleep(Duration::from_millis(2));
    fs::write(&new, [2u8; 10]).expect("new file");

    let report = prune_cache_at(
        CacheKind::RenderedBackground,
        CachePrunePolicy {
            max_bytes: 10,
            max_age: Duration::from_secs(60),
        },
        Some(&root),
        SystemTime::now(),
    )
    .expect("prune");

    assert_eq!(report.scanned_files, 2);
    assert_eq!(report.removed_files, 1);
    assert_eq!(report.removed_bytes, 10);
    assert_eq!(report.retained_bytes, 10);
    assert!(!old.exists());
    assert!(new.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn prunes_render_cache_entries_by_age() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-render-prune-age-test-{unique}"));
    let cache_dir = super::super::root(Some(&root), "backgrounds").expect("cache root");
    fs::create_dir_all(&cache_dir).expect("cache dir");

    let expired = cache_dir.join("expired.argb");
    fs::write(&expired, [1u8; 10]).expect("expired file");

    let report = prune_cache_at(
        CacheKind::RenderedBackground,
        CachePrunePolicy {
            max_bytes: 1024,
            max_age: Duration::from_secs(60),
        },
        Some(&root),
        SystemTime::now() + Duration::from_secs(61),
    )
    .expect("prune");

    assert_eq!(report.scanned_files, 1);
    assert_eq!(report.removed_files, 1);
    assert_eq!(report.removed_bytes, 10);
    assert_eq!(report.retained_bytes, 0);
    assert!(!expired.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn each_cache_kind_has_independent_limits_and_ignores_other_files() {
    let root = std::env::temp_dir().join(format!("veila-prune-kinds-{}", std::process::id()));
    let now = SystemTime::now();
    let policy = CachePrunePolicy {
        max_bytes: 10,
        max_age: Duration::from_secs(60),
    };
    for kind in [
        CacheKind::RenderedBackground,
        CacheKind::SourceImage,
        CacheKind::Avatar,
    ] {
        let dir = super::super::root(Some(&root), kind.directory()).unwrap();
        fs::create_dir_all(&dir).unwrap();
        for (name, age) in [("expired", 120), ("old", 30), ("new", 0)] {
            let path = dir.join(format!("{name}.{}", kind.extension()));
            fs::write(&path, [0; 10]).unwrap();
            fs::File::open(path)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(now - Duration::from_secs(age)))
                .unwrap();
        }
        fs::write(dir.join("active.tmp"), [0; 100]).unwrap();
        fs::write(dir.join("unrelated.txt"), [0; 100]).unwrap();
        fs::create_dir(dir.join(format!("directory.{}", kind.extension()))).unwrap();
        std::os::unix::fs::symlink(
            dir.join("unrelated.txt"),
            dir.join(format!("link.{}", kind.extension())),
        )
        .unwrap();
    }
    for kind in [
        CacheKind::RenderedBackground,
        CacheKind::SourceImage,
        CacheKind::Avatar,
    ] {
        let report = prune_cache_at(kind, policy, Some(&root), now).unwrap();
        assert_eq!(report.scanned_files, 3);
        assert_eq!(report.removed_files, 2);
        assert_eq!(report.retained_bytes, 10);
        let dir = super::super::root(Some(&root), kind.directory()).unwrap();
        assert!(dir.join(format!("new.{}", kind.extension())).exists());
        assert!(dir.join("active.tmp").exists());
        assert!(dir.join("unrelated.txt").exists());
        assert!(
            dir.join(format!("link.{}", kind.extension()))
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            prune_cache_at(kind, policy, Some(&root), now)
                .unwrap()
                .removed_files,
            0
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_cache_directories_need_no_creation() {
    let root = std::env::temp_dir().join(format!("veila-prune-missing-{}", std::process::id()));
    let report = prune_cache_at(
        CacheKind::Avatar,
        CachePrunePolicy {
            max_bytes: 0,
            max_age: Duration::ZERO,
        },
        Some(&root),
        SystemTime::now(),
    )
    .unwrap();
    assert_eq!(report.scanned_files, 0);
    assert!(!root.exists());
}

#[test]
fn abandoned_temporary_files_are_pruned_without_touching_recent_writes() {
    let root = std::env::temp_dir().join(format!("veila-prune-temp-{}", std::process::id()));
    let now = SystemTime::now();
    for kind in [
        CacheKind::RenderedBackground,
        CacheKind::SourceImage,
        CacheKind::Avatar,
    ] {
        let dir = super::super::root(Some(&root), kind.directory()).unwrap();
        fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join(format!(".0123456789abcdef.{}.tmp", kind.extension()));
        let abandoned = dir.join(format!(".0123456789abcdef.{}.123.4.tmp", kind.extension()));
        let recent = dir.join(format!(".0123456789abcdef.{}.123.5.tmp", kind.extension()));
        let unrelated = dir.join(".notes.tmp");
        for path in [&legacy, &abandoned, &unrelated] {
            fs::write(path, [0; 10]).unwrap();
            fs::File::open(path)
                .unwrap()
                .set_times(
                    fs::FileTimes::new()
                        .set_modified(now - super::STALE_TEMP_AGE - Duration::from_secs(1)),
                )
                .unwrap();
        }
        fs::write(&recent, [0; 10]).unwrap();
        let report = prune_cache_at(
            kind,
            CachePrunePolicy {
                max_bytes: 0,
                max_age: Duration::ZERO,
            },
            Some(&root),
            now,
        )
        .unwrap();
        assert_eq!(report.removed_files, 2);
        assert!(!legacy.exists());
        assert!(!abandoned.exists());
        assert!(recent.exists());
        assert!(unrelated.exists());
    }
    fs::remove_dir_all(root).unwrap();
}
