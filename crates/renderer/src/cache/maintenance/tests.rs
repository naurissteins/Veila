use super::*;
use std::{
    fs,
    path::Path,
    process::Command,
    time::{Instant, SystemTime},
};

#[test]
fn one_worker_prunes_all_families_after_the_delay() {
    if let Some(root) = std::env::var_os("VEILA_TEST_CACHE_MAINTENANCE") {
        check(Path::new(&root));
        return;
    }
    let root = std::env::temp_dir().join(format!("veila-prune-worker-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "cache::maintenance::tests::one_worker_prunes_all_families_after_the_delay",
            "--nocapture",
        ])
        .env("VEILA_TEST_CACHE_MAINTENANCE", &root)
        .env("XDG_CACHE_HOME", &root)
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

fn check(root: &Path) {
    let mut expired = Vec::new();
    let mut oversized = Vec::new();
    for (kind, extension, budget) in [
        (CacheKind::RenderedBackground, "argb", 768),
        (CacheKind::SourceImage, "rgba", 512),
        (CacheKind::Avatar, "rgba", 512),
    ] {
        let dir = root.join("veila").join(kind.directory());
        fs::create_dir_all(&dir).unwrap();
        let old = dir.join(format!("old.{extension}"));
        fs::write(&old, b"old").unwrap();
        fs::File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_times(
                fs::FileTimes::new()
                    .set_modified(SystemTime::now() - Duration::from_secs(15 * 24 * 60 * 60)),
            )
            .unwrap();
        expired.push(old);
        let huge = dir.join(format!("huge.{extension}"));
        fs::File::create(&huge)
            .unwrap()
            .set_len(budget * 1024 * 1024 + 1)
            .unwrap();
        oversized.push(huge);
        fs::write(dir.join("unrelated.txt"), b"keep").unwrap();
    }
    spawn_cache_maintenance(Duration::from_millis(100));
    spawn_cache_maintenance(Duration::ZERO);
    assert!(expired.iter().all(|p| p.exists()));
    let start = Instant::now();
    while expired.iter().chain(&oversized).any(|p| p.exists()) {
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(10));
    }
    for dir in ["backgrounds", "source-images", "avatars"] {
        assert!(root.join("veila").join(dir).join("unrelated.txt").exists());
    }
    let workers = fs::read_dir("/proc/self/task")
        .unwrap()
        .filter_map(|e| fs::read_to_string(e.ok()?.path().join("comm")).ok())
        .filter(|name| name.trim() == "veila-cache-pru")
        .count();
    assert_eq!(workers, 1);
}
