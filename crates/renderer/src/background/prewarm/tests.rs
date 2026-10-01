use std::{fs, path::Path, path::PathBuf, process::Command};

use super::FileBackgroundPrewarm;
use crate::{ClearColor, FrameSize, SoftwareBuffer, background::SourceCacheStatus};

const COLOR: ClearColor = ClearColor::opaque(37, 59, 83);
const SIZE: FrameSize = FrameSize::new(16, 8);

#[test]
fn cold_source_decode_is_reused_without_reading_its_cache() {
    let Some(root) = isolated("cold_source_decode_is_reused_without_reading_its_cache") else {
        return;
    };
    let path = wallpaper(&root);
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());
    assert_eq!(job.prewarm_source().unwrap(), SourceCacheStatus::Warmed);
    let cache = source_cache(&root);
    fs::write(&cache, b"corrupt source cache").unwrap();

    assert_eq!(job.render(SIZE).unwrap(), expected(SIZE));
    assert_eq!(fs::read(cache).unwrap(), b"corrupt source cache");
}

#[test]
fn warm_source_is_loaded_once_for_multiple_render_stages() {
    let Some(root) = isolated("warm_source_is_loaded_once_for_multiple_render_stages") else {
        return;
    };
    let path = wallpaper(&root);
    FileBackgroundPrewarm::new(&path, COLOR, Default::default())
        .prewarm_source()
        .unwrap();
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());
    assert_eq!(job.prewarm_source().unwrap(), SourceCacheStatus::Hit);
    assert!(job.asset.is_none());
    assert_eq!(job.prewarm_rendered(&[SIZE, SIZE]).unwrap().warmed_sizes, 1);
    fs::write(source_cache(&root), b"corrupt source cache").unwrap();

    let next = FrameSize::new(32, 16);
    assert_eq!(job.render(next).unwrap(), expected(next));
    assert_eq!(
        fs::read(source_cache(&root)).unwrap(),
        b"corrupt source cache"
    );
}

#[test]
fn complete_cache_hits_keep_source_pixels_unloaded() {
    let Some(root) = isolated("complete_cache_hits_keep_source_pixels_unloaded") else {
        return;
    };
    let path = wallpaper(&root);
    FileBackgroundPrewarm::new(&path, COLOR, Default::default())
        .prewarm_rendered(&[SIZE])
        .unwrap();
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());
    assert_eq!(job.prewarm_source().unwrap(), SourceCacheStatus::Hit);
    let report = job.prewarm_rendered(&[SIZE, SIZE]).unwrap();

    assert_eq!(report.cache_hits, 1);
    assert_eq!(report.warmed_sizes, 0);
    assert!(job.asset.is_none());
}

#[test]
fn empty_render_jobs_do_not_open_missing_sources() {
    let Some(root) = isolated("empty_render_jobs_do_not_open_missing_sources") else {
        return;
    };
    let path = root.join("missing.png");
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());
    let report = job.prewarm_rendered(&[]).unwrap();

    assert_eq!(report.cache_hits, 0);
    assert_eq!(report.warmed_sizes, 0);
    assert!(job.asset.is_none());
}

#[test]
fn corrupt_source_cache_is_repaired_and_decode_is_retained() {
    let Some(root) = isolated("corrupt_source_cache_is_repaired_and_decode_is_retained") else {
        return;
    };
    let path = wallpaper(&root);
    FileBackgroundPrewarm::new(&path, COLOR, Default::default())
        .prewarm_source()
        .unwrap();
    let cache = source_cache(&root);
    fs::write(&cache, b"truncated").unwrap();
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());

    assert_eq!(job.prewarm_source().unwrap(), SourceCacheStatus::Warmed);
    assert!(fs::metadata(cache).unwrap().len() > 16);
    assert!(job.asset.is_some());
    assert_eq!(job.render(SIZE).unwrap(), expected(SIZE));
}

#[test]
fn source_cache_write_failure_is_still_reported() {
    let Some(root) = isolated("source_cache_write_failure_is_still_reported") else {
        return;
    };
    let path = wallpaper(&root);
    fs::write(root.join("cache"), b"not a directory").unwrap();
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());

    assert!(job.prewarm_source().is_err());
    assert!(job.asset.is_none());
}

#[test]
fn invalid_source_image_does_not_create_an_asset() {
    let Some(root) = isolated("invalid_source_image_does_not_create_an_asset") else {
        return;
    };
    let path = root.join("invalid.png");
    fs::write(&path, b"not an image").unwrap();
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());

    assert!(job.prewarm_source().is_err());
    assert!(job.asset.is_none());
}

#[test]
fn same_size_same_time_replacement_reloads_source_pixels() {
    let Some(root) = isolated("same_size_same_time_replacement_reloads_source_pixels") else {
        return;
    };
    let path = wallpaper(&root);
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());
    job.prewarm_source().unwrap();
    job.prewarm_rendered(&[SIZE]).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let replacement = root.join("replacement.png");
    let color = ClearColor::opaque(83, 59, 37);
    SoftwareBuffer::solid(FrameSize::new(8, 4), color)
        .unwrap()
        .save_png(&replacement)
        .unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        fs::metadata(&replacement).unwrap().len()
    );
    fs::File::options()
        .write(true)
        .open(&replacement)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    fs::rename(replacement, &path).unwrap();

    assert_eq!(job.prewarm_rendered(&[SIZE]).unwrap().warmed_sizes, 1);
    assert_eq!(
        job.render(SIZE).unwrap(),
        SoftwareBuffer::solid(SIZE, color).unwrap()
    );
}

#[test]
fn removed_source_does_not_reuse_retained_pixels() {
    let Some(root) = isolated("removed_source_does_not_reuse_retained_pixels") else {
        return;
    };
    let path = wallpaper(&root);
    let mut job = FileBackgroundPrewarm::new(&path, COLOR, Default::default());
    job.prewarm_source().unwrap();
    fs::remove_file(&path).unwrap();

    assert!(job.render(SIZE).is_err());
}

fn isolated(test: &str) -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("VEILA_TEST_FILE_PREWARM") {
        return Some(root.into());
    }
    let root =
        std::env::temp_dir().join(format!("veila-file-prewarm-{test}-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("background::prewarm::tests::{test}"),
            "--nocapture",
        ])
        .env("VEILA_TEST_FILE_PREWARM", &root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .output()
        .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    None
}

fn wallpaper(root: &Path) -> PathBuf {
    let path = root.join("wallpaper.png");
    expected(FrameSize::new(8, 4)).save_png(&path).unwrap();
    path
}

fn expected(size: FrameSize) -> SoftwareBuffer {
    SoftwareBuffer::solid(size, COLOR).unwrap()
}

fn source_cache(root: &Path) -> PathBuf {
    fs::read_dir(root.join("cache/veila/source-images"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
}
