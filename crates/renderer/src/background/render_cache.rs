use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::{FrameSize, RendererError, Result, SoftwareBuffer};

const CACHE_MAGIC: &[u8; 8] = b"KWYBG001";

pub(crate) fn load_cached_buffer(path: &Path, size: FrameSize) -> Result<Option<SoftwareBuffer>> {
    let cache_path = cache_path(path, size, None)?;
    let Ok(mut file) = fs::File::open(&cache_path) else {
        return Ok(None);
    };

    let mut header = [0u8; 16];
    file.read_exact(&mut header)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    if &header[..8] != CACHE_MAGIC {
        return Ok(None);
    }

    let cached_size = FrameSize::new(
        u32::from_le_bytes(header[8..12].try_into().expect("width slice")),
        u32::from_le_bytes(header[12..16].try_into().expect("height slice")),
    );
    if cached_size != size {
        return Ok(None);
    }

    let Some(byte_len) = size.byte_len() else {
        return Err(RendererError::InvalidFrameSize(size));
    };
    let mut pixels = vec![0; byte_len];
    file.read_exact(&mut pixels)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;

    Ok(Some(SoftwareBuffer::from_argb8888_pixels(size, pixels)?))
}

pub(crate) fn store_cached_buffer(
    path: &Path,
    size: FrameSize,
    buffer: &SoftwareBuffer,
) -> Result<()> {
    let cache_path = cache_path(path, size, None)?;
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
            .unwrap_or("buffer")
    ));
    let mut file = fs::File::create(&temp_path)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(CACHE_MAGIC)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(&size.width.to_le_bytes())
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(&size.height.to_le_bytes())
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    file.write_all(buffer.pixels())
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

fn cache_path(path: &Path, size: FrameSize, cache_home: Option<&Path>) -> Result<PathBuf> {
    let metadata = fs::metadata(path)
        .map_err(image::ImageError::from)
        .map_err(RendererError::from)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let key = stable_hash(format!(
        "{}:{}:{}:{}x{}",
        path.display(),
        metadata.len(),
        modified,
        size.width,
        size.height
    ));

    Ok(cache_root(cache_home)?.join(format!("{key:016x}.argb")))
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

    Ok(base.join("veila").join("backgrounds"))
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

    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    use super::cache_path;

    #[test]
    fn round_trips_rendered_background_buffers() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veila-render-cache-test-{unique}"));
        fs::create_dir_all(&root).expect("cache root");

        let wallpaper = root.join("wallpaper.jpg");
        fs::write(&wallpaper, b"stub").expect("wallpaper file");

        let size = FrameSize::new(2, 1);
        let buffer = SoftwareBuffer::solid(size, ClearColor::opaque(12, 16, 24)).expect("buffer");
        store_cached_buffer_at(&wallpaper, size, &buffer, &root).expect("store");

        let loaded = load_cached_buffer_at(&wallpaper, size, &root)
            .expect("load")
            .expect("cached buffer");
        assert_eq!(loaded, buffer);

        let _ = fs::remove_dir_all(root);
    }

    fn load_cached_buffer_at(
        wallpaper: &Path,
        size: FrameSize,
        cache_home: &Path,
    ) -> crate::Result<Option<SoftwareBuffer>> {
        let cache_path = cache_path(wallpaper, size, Some(cache_home))?;
        let Ok(mut file) = fs::File::open(&cache_path) else {
            return Ok(None);
        };

        let mut header = [0u8; 16];
        file.read_exact(&mut header).expect("header");
        let mut pixels = vec![0; size.byte_len().expect("byte len")];
        file.read_exact(&mut pixels).expect("pixels");

        Ok(Some(
            SoftwareBuffer::from_argb8888_pixels(
                FrameSize::new(
                    u32::from_le_bytes(header[8..12].try_into().expect("width")),
                    u32::from_le_bytes(header[12..16].try_into().expect("height")),
                ),
                pixels,
            )
            .expect("buffer"),
        ))
    }

    fn store_cached_buffer_at(
        wallpaper: &Path,
        size: FrameSize,
        buffer: &SoftwareBuffer,
        cache_home: &Path,
    ) -> crate::Result<()> {
        let cache_path = cache_path(wallpaper, size, Some(cache_home))?;
        let cache_dir = cache_path.parent().expect("cache dir");
        fs::create_dir_all(cache_dir).expect("cache dir");
        let mut file = fs::File::create(cache_path).expect("cache file");
        file.write_all(super::CACHE_MAGIC).expect("magic");
        file.write_all(&size.width.to_le_bytes()).expect("width");
        file.write_all(&size.height.to_le_bytes()).expect("height");
        file.write_all(buffer.pixels()).expect("pixels");
        Ok(())
    }
}
