use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use image::RgbaImage;

use crate::{FrameSize, RendererError, Result};

const CACHE_MAGIC: &[u8; 8] = b"KWYIMG01";

pub(crate) fn load_cached_rgba(path: &Path) -> Result<Option<RgbaImage>> {
    let cache_path = cache_path(path, None)?;
    let Ok(mut file) = fs::File::open(&cache_path) else {
        return Ok(None);
    };

    let mut header = [0u8; 16];
    file.read_exact(&mut header)
        .map_err(image::ImageError::from)?;
    if &header[..8] != CACHE_MAGIC {
        return Ok(None);
    }

    let size = FrameSize::new(
        u32::from_le_bytes(header[8..12].try_into().expect("width slice")),
        u32::from_le_bytes(header[12..16].try_into().expect("height slice")),
    );
    let Some(byte_len) = size.byte_len() else {
        return Err(RendererError::InvalidFrameSize(size));
    };

    let mut pixels = vec![0; byte_len];
    file.read_exact(&mut pixels)
        .map_err(image::ImageError::from)?;

    RgbaImage::from_raw(size.width, size.height, pixels)
        .ok_or(RendererError::InvalidFrameSize(size))
        .map(Some)
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
    let key = stable_hash(format!(
        "{}:{}:{}",
        path.display(),
        metadata.len(),
        modified
    ));

    Ok(cache_root(cache_home)?.join(format!("{key:016x}.rgba")))
}

fn cache_root(cache_home: Option<&Path>) -> Result<PathBuf> {
    let base = cache_home
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| {
            RendererError::Image(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "failed to resolve XDG cache directory",
            )))
        })?;

    Ok(base.join("kwylock").join("source-images"))
}

fn stable_hash(input: String) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;

    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    use image::{Rgba, RgbaImage};

    use super::cache_path;

    #[test]
    fn round_trips_decoded_source_cache() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kwylock-source-cache-test-{unique}"));
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

    fn load_cached_rgba_at(
        path: &Path,
        cache_home: Option<&Path>,
    ) -> super::Result<Option<RgbaImage>> {
        let cache_path = cache_path(path, cache_home)?;
        let Ok(mut file) = fs::File::open(&cache_path) else {
            return Ok(None);
        };

        let mut header = [0u8; 16];
        file.read_exact(&mut header).expect("header");
        let width = u32::from_le_bytes(header[8..12].try_into().expect("width"));
        let height = u32::from_le_bytes(header[12..16].try_into().expect("height"));
        let mut pixels = vec![0; (width as usize) * (height as usize) * 4];
        file.read_exact(&mut pixels).expect("pixels");

        Ok(RgbaImage::from_raw(width, height, pixels))
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
