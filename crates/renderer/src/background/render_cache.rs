use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::{FrameSize, RendererError, Result, SoftwareBuffer};

use super::{BackgroundTreatment, GeneratedBackground};

const CACHE_MAGIC: &[u8; 8] = b"KWYBG001";

#[derive(Debug, Clone, Copy)]
enum CacheSource<'a> {
    Path(&'a Path),
    Generated(GeneratedBackground),
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
        treatment.blur_radius, treatment.dim_strength, treatment.tint, treatment.tint_opacity
    ));
    let key = stable_hash(format!("{key}:{}", variant.unwrap_or_default()));

    Ok(cache_root(cache_home)?.join(format!("{key:016x}.argb")))
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
        }),
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
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{ClearColor, FrameSize, SoftwareBuffer};

    use super::{BackgroundTreatment, CacheSource, GeneratedBackground, cache_path};
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
