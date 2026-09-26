use crate::cache::stable_hash;

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use image::RgbaImage;

use crate::{FrameSize, RendererError, Result};

const CACHE_MAGIC: &[u8; 8] = b"KWYIMG01";
const MAX_CACHED_DIMENSION: u32 = 16_384;

pub(crate) fn load_cached_rgba(path: &Path) -> Result<Option<RgbaImage>> {
    load_cached_rgba_at(path, None)
}

fn load_cached_rgba_at(path: &Path, cache_home: Option<&Path>) -> Result<Option<RgbaImage>> {
    let cache_path = cache_path(path, cache_home)?;
    let Ok(mut file) = fs::File::open(&cache_path) else {
        return Ok(None);
    };

    let image = (|| {
        let mut header = [0u8; 16];
        file.read_exact(&mut header).ok()?;
        if &header[..8] != CACHE_MAGIC {
            return None;
        }

        let width = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        let height = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
        if width == 0
            || height == 0
            || width > MAX_CACHED_DIMENSION
            || height > MAX_CACHED_DIMENSION
        {
            return None;
        }
        let byte_len = FrameSize::new(width, height).byte_len()?;
        let expected_len = 16u64.checked_add(u64::try_from(byte_len).ok()?)?;
        if file.metadata().ok()?.len() != expected_len {
            return None;
        }

        let mut pixels = vec![0; byte_len];
        file.read_exact(&mut pixels).ok()?;
        RgbaImage::from_raw(width, height, pixels)
    })();
    if image.is_none() {
        let _ = fs::remove_file(cache_path);
    }
    Ok(image)
}

pub(crate) fn store_cached_rgba(path: &Path, image: &RgbaImage) -> Result<()> {
    let cache_path = cache_path(path, None)?;
    let Some(cache_dir) = cache_path.parent() else {
        return Err(RendererError::Image(image::ImageError::IoError(
            std::io::Error::other("cache path has no parent"),
        )));
    };
    fs::create_dir_all(cache_dir)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;

    let temp_path = cache_dir.join(format!(
        ".{}.tmp",
        cache_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("source")
    ));
    let mut file = fs::File::create(&temp_path)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(CACHE_MAGIC)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(&image.width().to_le_bytes())
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(&image.height().to_le_bytes())
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(image.as_raw())
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.flush()
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    fs::rename(&temp_path, &cache_path)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;

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
        io::Write,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    use image::{Rgba, RgbaImage};

    use super::{CACHE_MAGIC, cache_path, load_cached_rgba_at};

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
        assert!(!path.exists());

        let _ = fs::remove_dir_all(root);
    }

    fn store_cached_rgba_at(
        path: &Path,
        image: &RgbaImage,
        cache_home: Option<&Path>,
    ) -> super::Result<()> {
        let cache_path = cache_path(path, cache_home)?;
        let cache_dir = cache_path.parent().expect("cache dir");
        fs::create_dir_all(cache_dir).expect("cache dir");
        let mut file = fs::File::create(cache_path).expect("cache file");
        file.write_all(super::CACHE_MAGIC).expect("magic");
        file.write_all(&image.width().to_le_bytes()).expect("width");
        file.write_all(&image.height().to_le_bytes())
            .expect("height");
        file.write_all(image.as_raw()).expect("pixels");
        Ok(())
    }
}
