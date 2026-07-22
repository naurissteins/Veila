use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{FrameSize, RendererError, Result, SoftwareBuffer};

use super::{BackgroundTreatment, GeneratedBackground};

const CACHE_MAGIC: &[u8; 8] = b"KWYBG001";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCachePrunePolicy {
    pub max_bytes: u64,
    pub max_age: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderCachePruneReport {
    pub scanned_files: usize,
    pub removed_files: usize,
    pub removed_bytes: u64,
    pub retained_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
enum CacheSource<'a> {
    Path(&'a Path),
    Generated(GeneratedBackground),
}

#[derive(Debug)]
struct PruneEntry {
    path: PathBuf,
    byte_len: u64,
    modified: SystemTime,
}

pub(crate) fn load_cached_buffer(
    path: &Path,
    size: FrameSize,
    treatment: BackgroundTreatment,
) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer_for_source(CacheSource::Path(path), size, treatment, None)
}

pub(crate) fn load_cached_buffer_for_generated(
    generated: GeneratedBackground,
    size: FrameSize,
    treatment: BackgroundTreatment,
) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer_for_source(CacheSource::Generated(generated), size, treatment, None)
}

pub(crate) fn load_cached_buffer_with_variant(
    path: &Path,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: Option<&str>,
) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer_for_source(CacheSource::Path(path), size, treatment, variant)
}

pub(crate) fn load_cached_buffer_for_generated_with_variant(
    generated: GeneratedBackground,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: Option<&str>,
) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer_for_source(CacheSource::Generated(generated), size, treatment, variant)
}

fn load_cached_buffer_for_source(
    source: CacheSource<'_>,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: Option<&str>,
) -> Result<Option<SoftwareBuffer>> {
    let cache_path = cache_path(source, size, treatment, variant, None)?;
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
        // Infallible: the header was read with read_exact into a fixed 16-byte array
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
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
) -> Result<()> {
    store_cached_buffer_for_source(CacheSource::Path(path), size, treatment, buffer, None)
}

pub(crate) fn store_cached_buffer_for_generated(
    generated: GeneratedBackground,
    size: FrameSize,
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
) -> Result<()> {
    store_cached_buffer_for_source(
        CacheSource::Generated(generated),
        size,
        treatment,
        buffer,
        None,
    )
}

pub(crate) fn store_cached_buffer_with_variant(
    path: &Path,
    size: FrameSize,
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
    variant: Option<&str>,
) -> Result<()> {
    store_cached_buffer_for_source(CacheSource::Path(path), size, treatment, buffer, variant)
}

pub(crate) fn store_cached_buffer_for_generated_with_variant(
    generated: GeneratedBackground,
    size: FrameSize,
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
    variant: Option<&str>,
) -> Result<()> {
    store_cached_buffer_for_source(
        CacheSource::Generated(generated),
        size,
        treatment,
        buffer,
        variant,
    )
}

fn store_cached_buffer_for_source(
    source: CacheSource<'_>,
    size: FrameSize,
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
    variant: Option<&str>,
) -> Result<()> {
    let cache_path = cache_path(source, size, treatment, variant, None)?;
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

fn cache_path(
    source: CacheSource<'_>,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: Option<&str>,
    cache_home: Option<&Path>,
) -> Result<PathBuf> {
    let key = stable_hash(cache_source_key(source, size)?);
    let key = stable_hash(format!(
        "{key}:{:?}:{:?}:{:?}:{:?}",
        treatment.blur_radius, treatment.dim_strength, treatment.tint, treatment.scaling
    ));
    let key = stable_hash(format!("{key}:{}", variant.unwrap_or_default()));

    Ok(cache_root(cache_home)?.join(format!("{key:016x}.argb")))
}

pub fn prune_render_cache(policy: RenderCachePrunePolicy) -> Result<RenderCachePruneReport> {
    prune_render_cache_at(policy, None, SystemTime::now())
}

fn prune_render_cache_at(
    policy: RenderCachePrunePolicy,
    cache_home: Option<&Path>,
    now: SystemTime,
) -> Result<RenderCachePruneReport> {
    let root = cache_root(cache_home)?;
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RenderCachePruneReport::default());
        }
        Err(error) => return Err(RendererError::Io(error)),
    };

    let mut report = RenderCachePruneReport::default();
    let mut retained = Vec::new();

    for entry in entries {
        let entry = entry.map_err(RendererError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("argb") {
            continue;
        }

        let metadata = entry.metadata().map_err(RendererError::Io)?;
        if !metadata.is_file() {
            continue;
        }

        report.scanned_files += 1;
        let byte_len = metadata.len();
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);

        if now
            .duration_since(modified)
            .is_ok_and(|age| age > policy.max_age)
        {
            remove_pruned_file(&path, byte_len, &mut report)?;
        } else {
            retained.push(PruneEntry {
                path,
                byte_len,
                modified,
            });
        }
    }

    let mut retained_bytes = retained.iter().map(|entry| entry.byte_len).sum::<u64>();
    retained.sort_by_key(|entry| entry.modified);

    for entry in retained {
        if retained_bytes <= policy.max_bytes {
            break;
        }

        remove_pruned_file(&entry.path, entry.byte_len, &mut report)?;
        retained_bytes = retained_bytes.saturating_sub(entry.byte_len);
    }

    report.retained_bytes = retained_bytes;
    Ok(report)
}

fn remove_pruned_file(
    path: &Path,
    byte_len: u64,
    report: &mut RenderCachePruneReport,
) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => {
            report.removed_files += 1;
            report.removed_bytes = report.removed_bytes.saturating_add(byte_len);
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(RendererError::Io(error)),
    }
}

fn cache_source_key(source: CacheSource<'_>, size: FrameSize) -> Result<String> {
    match source {
        CacheSource::Path(path) => {
            let metadata = fs::metadata(path)
                .map_err(image::ImageError::from)
                .map_err(RendererError::from)?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or_default();
            Ok(format!(
                "image:v1:{}:{}:{}:{}x{}",
                path.display(),
                metadata.len(),
                modified,
                size.width,
                size.height
            ))
        }
        CacheSource::Generated(generated) => Ok(match generated {
            GeneratedBackground::Gradient(gradient) => format!(
                "gradient:v1:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}x{}",
                gradient.top_left.red,
                gradient.top_left.green,
                gradient.top_left.blue,
                gradient.top_left.alpha,
                gradient.top_right.red,
                gradient.top_right.green,
                gradient.top_right.blue,
                gradient.top_right.alpha,
                gradient.bottom_left.red,
                gradient.bottom_left.green,
                gradient.bottom_left.blue,
                gradient.bottom_left.alpha,
                gradient.bottom_right.red,
                gradient.bottom_right.green,
                gradient.bottom_right.blue,
                gradient.bottom_right.alpha,
                size.width,
                size.height
            ),
            GeneratedBackground::Radial(radial) => format!(
                "radial:v1:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}x{}",
                radial.center.red,
                radial.center.green,
                radial.center.blue,
                radial.center.alpha,
                radial.edge.red,
                radial.edge.green,
                radial.edge.blue,
                radial.edge.alpha,
                radial.center_x,
                radial.center_y,
                radial.radius,
                size.width,
                size.height
            ),
            GeneratedBackground::Layered(layered) => format!(
                "layered:v1:{}:{}:{}:{}:{}x{}",
                layered_base_key(layered.base),
                layered_blob_key(layered.blobs[0]),
                layered_blob_key(layered.blobs[1]),
                layered_blob_key(layered.blobs[2]),
                size.width,
                size.height
            ),
        }),
    }
}

fn layered_base_key(base: super::BackgroundLayeredBase) -> String {
    match base {
        super::BackgroundLayeredBase::Solid(color) => {
            format!(
                "solid:{}:{}:{}:{}",
                color.red, color.green, color.blue, color.alpha
            )
        }
        super::BackgroundLayeredBase::Gradient(gradient) => format!(
            "gradient:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            gradient.top_left.red,
            gradient.top_left.green,
            gradient.top_left.blue,
            gradient.top_left.alpha,
            gradient.top_right.red,
            gradient.top_right.green,
            gradient.top_right.blue,
            gradient.top_right.alpha,
            gradient.bottom_left.red,
            gradient.bottom_left.green,
            gradient.bottom_left.blue,
            gradient.bottom_left.alpha,
            gradient.bottom_right.red,
            gradient.bottom_right.green,
            gradient.bottom_right.blue,
            gradient.bottom_right.alpha
        ),
        super::BackgroundLayeredBase::Radial(radial) => format!(
            "radial:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            radial.center.red,
            radial.center.green,
            radial.center.blue,
            radial.center.alpha,
            radial.edge.red,
            radial.edge.green,
            radial.edge.blue,
            radial.edge.alpha,
            radial.center_x,
            radial.center_y,
            radial.radius
        ),
    }
}

fn layered_blob_key(blob: Option<super::BackgroundLayeredBlob>) -> String {
    match blob {
        Some(blob) => format!(
            "blob:{}:{}:{}:{}:{}:{}:{}",
            blob.color.red,
            blob.color.green,
            blob.color.blue,
            blob.color.alpha,
            blob.x,
            blob.y,
            blob.size
        ),
        None => String::from("none"),
    }
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
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    use super::{
        super::BackgroundScaling, BackgroundTreatment, CacheSource, GeneratedBackground,
        RenderCachePrunePolicy, cache_path, prune_render_cache_at,
    };
    use crate::background::BackgroundGradient;

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
        store_cached_buffer_at(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment::default(),
            &buffer,
            None,
            &root,
        )
        .expect("store");

        let loaded = load_cached_buffer_at(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment::default(),
            None,
            &root,
        )
        .expect("load")
        .expect("cached buffer");
        assert_eq!(loaded, buffer);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn separates_variant_cache_entries() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veila-render-cache-variant-test-{unique}"));
        fs::create_dir_all(&root).expect("cache root");

        let wallpaper = root.join("wallpaper.jpg");
        fs::write(&wallpaper, b"stub").expect("wallpaper file");

        let size = FrameSize::new(2, 1);
        let base = SoftwareBuffer::solid(size, ClearColor::opaque(12, 16, 24)).expect("buffer");
        let layered = SoftwareBuffer::solid(size, ClearColor::opaque(40, 50, 60)).expect("buffer");
        store_cached_buffer_at(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment::default(),
            &base,
            None,
            &root,
        )
        .expect("store base");
        store_cached_buffer_at(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment::default(),
            &layered,
            Some("layer:v1"),
            &root,
        )
        .expect("store layered");

        let loaded_base = load_cached_buffer_at(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment::default(),
            None,
            &root,
        )
        .expect("load")
        .expect("cached buffer");
        let loaded_layered = load_cached_buffer_at(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment::default(),
            Some("layer:v1"),
            &root,
        )
        .expect("load")
        .expect("cached buffer");
        assert_eq!(loaded_base, base);
        assert_eq!(loaded_layered, layered);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn separates_generated_cache_entries() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veila-render-generated-cache-test-{unique}"));
        fs::create_dir_all(&root).expect("cache root");

        let generated = GeneratedBackground::Gradient(BackgroundGradient {
            top_left: ClearColor::opaque(255, 0, 0),
            top_right: ClearColor::opaque(0, 255, 0),
            bottom_left: ClearColor::opaque(0, 0, 255),
            bottom_right: ClearColor::opaque(255, 255, 255),
        });
        let size = FrameSize::new(2, 1);
        let base = SoftwareBuffer::solid(size, ClearColor::opaque(12, 16, 24)).expect("buffer");
        let layered = SoftwareBuffer::solid(size, ClearColor::opaque(40, 50, 60)).expect("buffer");
        store_cached_buffer_at(
            CacheSource::Generated(generated),
            size,
            BackgroundTreatment::default(),
            &base,
            None,
            &root,
        )
        .expect("store base");
        store_cached_buffer_at(
            CacheSource::Generated(generated),
            size,
            BackgroundTreatment::default(),
            &layered,
            Some("layer:v1"),
            &root,
        )
        .expect("store layered");

        let loaded_base = load_cached_buffer_at(
            CacheSource::Generated(generated),
            size,
            BackgroundTreatment::default(),
            None,
            &root,
        )
        .expect("load")
        .expect("cached buffer");
        let loaded_layered = load_cached_buffer_at(
            CacheSource::Generated(generated),
            size,
            BackgroundTreatment::default(),
            Some("layer:v1"),
            &root,
        )
        .expect("load")
        .expect("cached buffer");
        assert_eq!(loaded_base, base);
        assert_eq!(loaded_layered, layered);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn separates_cache_entries_by_background_scaling() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veila-render-scale-cache-test-{unique}"));
        fs::create_dir_all(&root).expect("cache root");

        let wallpaper = root.join("wallpaper.jpg");
        fs::write(&wallpaper, b"stub").expect("wallpaper file");
        let size = FrameSize::new(1920, 1080);

        let fill = cache_path(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment {
                scaling: BackgroundScaling::Fill,
                ..BackgroundTreatment::default()
            },
            None,
            Some(&root),
        )
        .expect("fill key");
        let fit = cache_path(
            CacheSource::Path(&wallpaper),
            size,
            BackgroundTreatment {
                scaling: BackgroundScaling::Fit,
                ..BackgroundTreatment::default()
            },
            None,
            Some(&root),
        )
        .expect("fit key");

        assert_ne!(fill, fit);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prunes_oldest_render_cache_entries_to_size_limit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veila-render-prune-size-test-{unique}"));
        let cache_dir = super::cache_root(Some(&root)).expect("cache root");
        fs::create_dir_all(&cache_dir).expect("cache dir");

        let old = cache_dir.join("old.argb");
        let new = cache_dir.join("new.argb");
        fs::write(&old, [1u8; 10]).expect("old file");
        std::thread::sleep(Duration::from_millis(2));
        fs::write(&new, [2u8; 10]).expect("new file");

        let report = prune_render_cache_at(
            RenderCachePrunePolicy {
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
        let cache_dir = super::cache_root(Some(&root)).expect("cache root");
        fs::create_dir_all(&cache_dir).expect("cache dir");

        let expired = cache_dir.join("expired.argb");
        fs::write(&expired, [1u8; 10]).expect("expired file");

        let report = prune_render_cache_at(
            RenderCachePrunePolicy {
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

    fn load_cached_buffer_at(
        source: CacheSource<'_>,
        size: FrameSize,
        treatment: BackgroundTreatment,
        variant: Option<&str>,
        cache_home: &Path,
    ) -> crate::Result<Option<SoftwareBuffer>> {
        let cache_path = cache_path(source, size, treatment, variant, Some(cache_home))?;
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
        source: CacheSource<'_>,
        size: FrameSize,
        treatment: BackgroundTreatment,
        buffer: &SoftwareBuffer,
        variant: Option<&str>,
        cache_home: &Path,
    ) -> crate::Result<()> {
        let cache_path = cache_path(source, size, treatment, variant, Some(cache_home))?;
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
