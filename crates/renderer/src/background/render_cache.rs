use crate::cache::stable_hash;

use std::{
    fs,
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
    let Ok(cache_path) = cache_path(source, size, treatment, variant, None) else {
        return Ok(None);
    };
    Ok(read_buffer(&cache_path, size))
}

fn read_buffer(path: &Path, size: FrameSize) -> Option<SoftwareBuffer> {
    let (_, pixels) = crate::cache::image::read_pixels(path, CACHE_MAGIC, u32::MAX, Some(size))?;
    SoftwareBuffer::from_argb8888_pixels(size, pixels).ok()
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
    write_buffer(&cache_path, size, buffer)
}

fn write_buffer(path: &Path, size: FrameSize, buffer: &SoftwareBuffer) -> Result<()> {
    if buffer.size() != size {
        return Err(RendererError::InvalidFrameSize(size));
    }
    crate::cache::image::write_pixels(path, CACHE_MAGIC, size, buffer.pixels())
        .map_err(image::ImageError::from)?;
    Ok(())
}

fn cache_path(
    source: CacheSource<'_>,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: Option<&str>,
    cache_home: Option<&Path>,
) -> Result<PathBuf> {
    let key = stable_hash(&cache_source_key(source, size)?);
    let key = stable_hash(&format!(
        "{key}:{:?}:{:?}:{:?}:{:?}",
        treatment.blur_radius, treatment.dim_strength, treatment.tint, treatment.scaling
    ));
    let key = stable_hash(&format!("{key}:{}", variant.unwrap_or_default()));

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
    Ok(crate::cache::root(cache_home, "backgrounds").map_err(image::ImageError::IoError)?)
}

#[cfg(test)]
mod tests;
