use crate::cache::stable_hash;

use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use image::RgbaImage;

use crate::{FrameSize, Result};

const CACHE_MAGIC: &[u8; 8] = b"KWYIMG01";
const MAX_CACHED_DIMENSION: u32 = 16_384;

pub(super) fn has_cached_rgba(path: &Path) -> bool {
    cache_path(path, None).ok().is_some_and(|path| {
        crate::cache::image::cached_size(&path, CACHE_MAGIC, MAX_CACHED_DIMENSION).is_some()
    })
}

pub(crate) fn load_cached_rgba(path: &Path) -> Result<Option<RgbaImage>> {
    load_cached_rgba_at(path, None)
}

fn load_cached_rgba_at(path: &Path, cache_home: Option<&Path>) -> Result<Option<RgbaImage>> {
    let Ok(cache_path) = cache_path(path, cache_home) else {
        return Ok(None);
    };
    Ok(
        crate::cache::image::read_pixels(&cache_path, CACHE_MAGIC, MAX_CACHED_DIMENSION, None)
            .and_then(|(size, pixels)| RgbaImage::from_raw(size.width, size.height, pixels)),
    )
}

pub(crate) fn store_cached_rgba(path: &Path, image: &RgbaImage) -> Result<()> {
    store_cached_rgba_at(path, image, None)
}

fn store_cached_rgba_at(path: &Path, image: &RgbaImage, cache_home: Option<&Path>) -> Result<()> {
    let cache_path = cache_path(path, cache_home)?;
    crate::cache::image::write_pixels(
        &cache_path,
        CACHE_MAGIC,
        FrameSize::new(image.width(), image.height()),
        image.as_raw(),
    )
    .map_err(image::ImageError::from)?;
    Ok(())
}

fn cache_path(path: &Path, cache_home: Option<&Path>) -> Result<PathBuf> {
    let metadata = fs::metadata(path).map_err(image::ImageError::from)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let key = stable_hash(&format!(
        "{}:{}:{}",
        path.display(),
        metadata.len(),
        modified
    ));

    Ok(cache_root(cache_home)?.join(format!("{key:016x}.rgba")))
}

fn cache_root(cache_home: Option<&Path>) -> Result<PathBuf> {
    Ok(crate::cache::root(cache_home, "source-images").map_err(image::ImageError::IoError)?)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use image::{Rgba, RgbaImage};

    use super::{CACHE_MAGIC, cache_path, load_cached_rgba_at, store_cached_rgba_at};

    #[test]
    fn round_trips_decoded_source_cache() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veila-source-cache-test-{unique}"));
        fs::create_dir_all(&root).expect("cache root");

        let wallpaper = root.join("wallpaper.png");
        fs::write(&wallpaper, b"stub").expect("wallpaper file");

        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
        store_cached_rgba_at(&wallpaper, &image, Some(&root)).expect("store");

        let loaded = load_cached_rgba_at(&wallpaper, Some(&root))
            .expect("load")
            .expect("cached image");
        assert_eq!(loaded, image);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_truncated_source_cache_before_allocating_pixels() {
        let root =
            std::env::temp_dir().join(format!("veila-source-cache-invalid-{}", std::process::id()));
        fs::create_dir_all(root.join("veila/source-images")).expect("cache dir");
        let wallpaper = root.join("wallpaper.png");
        fs::write(&wallpaper, b"stub").expect("wallpaper file");
        let path = cache_path(&wallpaper, Some(&root)).expect("cache path");
        let mut cache = Vec::from(CACHE_MAGIC.as_slice());
        cache.extend_from_slice(&16_000u32.to_le_bytes());
        cache.extend_from_slice(&16_000u32.to_le_bytes());
        fs::write(&path, cache).expect("invalid cache");

        assert!(
            load_cached_rgba_at(&wallpaper, Some(&root))
                .expect("cache miss")
                .is_none()
        );
        assert!(path.exists());

        let _ = fs::remove_dir_all(root);
    }
}
