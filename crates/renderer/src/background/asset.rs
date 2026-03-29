use std::{path::Path, sync::Arc};

use image::RgbaImage;

use super::{
    BackgroundAsset, BackgroundKind, BackgroundTreatment, RenderCacheSummary, SourceCacheStatus,
    render::render_image,
    render_cache::{
        load_cached_buffer, load_cached_buffer_with_variant, store_cached_buffer,
        store_cached_buffer_with_variant,
    },
    source_cache::{load_cached_rgba, store_cached_rgba},
    treatment::apply_treatment,
};
use crate::{ClearColor, FrameSize, Result, SoftwareBuffer};

impl BackgroundAsset {
    pub fn load(
        path: Option<&Path>,
        fallback: ClearColor,
        treatment: BackgroundTreatment,
    ) -> Result<Self> {
        match path {
            Some(path) => Ok(Self {
                kind: BackgroundKind::Image(Arc::new(load_rgba_image(path)?)),
                treatment,
            }),
            None => Ok(Self {
                kind: BackgroundKind::Solid(fallback),
                treatment,
            }),
        }
    }

    pub fn render(&self, size: FrameSize) -> Result<SoftwareBuffer> {
        let mut buffer = match &self.kind {
            BackgroundKind::Solid(color) => SoftwareBuffer::solid(size, *color),
            BackgroundKind::Image(image) => render_image(image, size, self.treatment),
        }?;
        apply_treatment(&mut buffer, self.treatment);
        Ok(buffer)
    }
}

pub fn prewarm_source(path: &Path) -> Result<SourceCacheStatus> {
    if load_cached_rgba(path)?.is_some() {
        return Ok(SourceCacheStatus::Hit);
    }

    let image = image::open(path)?.to_rgba8();
    store_cached_rgba(path, &image)?;
    Ok(SourceCacheStatus::Warmed)
}

pub fn load_cached_render(
    path: &Path,
    size: FrameSize,
    treatment: BackgroundTreatment,
) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer(path, size, treatment)
}

pub fn load_cached_render_variant(
    path: &Path,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: &str,
) -> Result<Option<SoftwareBuffer>> {
    load_cached_buffer_with_variant(path, size, treatment, Some(variant))
}

pub fn store_cached_render(
    path: &Path,
    size: FrameSize,
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
) -> Result<()> {
    store_cached_buffer(path, size, treatment, buffer)
}

pub fn store_cached_render_variant(
    path: &Path,
    size: FrameSize,
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
    variant: &str,
) -> Result<()> {
    store_cached_buffer_with_variant(path, size, treatment, buffer, Some(variant))
}

pub fn prewarm_rendered(
    path: &Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    sizes: &[FrameSize],
) -> Result<RenderCacheSummary> {
    let unique_sizes = unique_sizes(sizes);
    let mut cache_hits = 0;
    let mut missing_sizes = Vec::new();

    for size in unique_sizes {
        if load_cached_render(path, size, treatment)?.is_some() {
            cache_hits += 1;
        } else {
            missing_sizes.push(size);
        }
    }

    if missing_sizes.is_empty() {
        return Ok(RenderCacheSummary {
            cache_hits,
            warmed_sizes: 0,
        });
    }

    let asset = BackgroundAsset::load(Some(path), fallback, treatment)?;
    for size in &missing_sizes {
        let buffer = asset.render(*size)?;
        store_cached_render(path, *size, treatment, &buffer)?;
    }

    Ok(RenderCacheSummary {
        cache_hits,
        warmed_sizes: missing_sizes.len(),
    })
}

pub(super) fn load_rgba_image(path: &Path) -> Result<RgbaImage> {
    if let Some(image) = load_cached_rgba(path)? {
        return Ok(image);
    }

    let image = image::open(path)?.to_rgba8();
    let _ = store_cached_rgba(path, &image);
    Ok(image)
}

pub(super) fn unique_sizes(sizes: &[FrameSize]) -> Vec<FrameSize> {
    let mut unique = Vec::with_capacity(sizes.len());

    for size in sizes {
        if !unique.contains(size) {
            unique.push(*size);
        }
    }

    unique
}
