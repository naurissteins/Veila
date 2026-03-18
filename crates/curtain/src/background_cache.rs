use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{Context, Result, anyhow};
use kwylock_renderer::{FrameSize, SoftwareBuffer};

const CACHE_MAGIC: &[u8; 8] = b"KWYBG001";

pub(crate) fn load_cached_buffer(path: &Path, size: FrameSize) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer_at(path, size, None)
}

fn load_cached_buffer_at(
    path: &Path,
    size: FrameSize,
    cache_home: Option<&Path>,
) -> Result<Option<SoftwareBuffer>> {
    let cache_path = cache_path(path, size, cache_home)?;
    let Ok(mut file) = fs::File::open(&cache_path) else {
        return Ok(None);
    };

    let mut header = [0u8; 16];
    file.read_exact(&mut header)
        .with_context(|| format!("failed to read {}", cache_path.display()))?;

    if &header[..8] != CACHE_MAGIC {
        return Ok(None);
    }

    let cached_size = FrameSize::new(
        u32::from_le_bytes(header[8..12].try_into().expect("slice size")),
        u32::from_le_bytes(header[12..16].try_into().expect("slice size")),
    );
    if cached_size != size {
        return Ok(None);
    }

    let Some(byte_len) = size.byte_len() else {
        return Err(anyhow!("invalid cached frame size {size:?}"));
    };
    let mut pixels = vec![0; byte_len];
    file.read_exact(&mut pixels)
        .with_context(|| format!("failed to read {}", cache_path.display()))?;

    Ok(Some(SoftwareBuffer::from_argb8888_pixels(size, pixels)?))
}

pub(crate) fn store_cached_buffer(
    path: &Path,
    size: FrameSize,
    buffer: &SoftwareBuffer,
) -> Result<()> {
    store_cached_buffer_at(path, size, buffer, None)
}

fn store_cached_buffer_at(
    path: &Path,
    size: FrameSize,
    buffer: &SoftwareBuffer,
    cache_home: Option<&Path>,
) -> Result<()> {
    let cache_path = cache_path(path, size, cache_home)?;
    let Some(cache_dir) = cache_path.parent() else {
        return Err(anyhow!("cache path has no parent"));
    };
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("failed to create {}", cache_dir.display()))?;

    let temp_path = cache_dir.join(format!(
        ".{}.tmp",
        cache_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("buffer")
    ));
    let mut file = fs::File::create(&temp_path)
        .with_context(|| format!("failed to create {}", temp_path.display()))?;
    file.write_all(CACHE_MAGIC)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    file.write_all(&size.width.to_le_bytes())
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    file.write_all(&size.height.to_le_bytes())
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    file.write_all(buffer.pixels())
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush {}", temp_path.display()))?;
    fs::rename(&temp_path, &cache_path).with_context(|| {
        format!(
            "failed to move cached background {} into place",
            cache_path.display()
        )
    })?;

    Ok(())
}

fn cache_path(path: &Path, size: FrameSize, cache_home: Option<&Path>) -> Result<PathBuf> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to stat wallpaper {}", path.display()))?;
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
        .ok_or_else(|| anyhow!("failed to resolve XDG cache directory"))?;

    Ok(base.join("kwylock").join("backgrounds"))
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
        time::{SystemTime, UNIX_EPOCH},
    };

    use kwylock_renderer::{ClearColor, FrameSize, SoftwareBuffer};

    use super::{load_cached_buffer_at, store_cached_buffer_at};

    #[test]
    fn round_trips_cached_background_buffers() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kwylock-cache-test-{unique}"));
        fs::create_dir_all(&root).expect("cache root");

        let wallpaper = root.join("wallpaper.jpg");
        fs::write(&wallpaper, b"stub").expect("wallpaper file");

        let size = FrameSize::new(2, 1);
        let buffer = SoftwareBuffer::solid(size, ClearColor::opaque(12, 16, 24)).expect("buffer");
        store_cached_buffer_at(&wallpaper, size, &buffer, Some(&root)).expect("store");

        let loaded = load_cached_buffer_at(&wallpaper, size, Some(&root))
            .expect("load")
            .expect("cached buffer");
        assert_eq!(loaded, buffer);

        let _ = fs::remove_dir_all(root);
    }
}
