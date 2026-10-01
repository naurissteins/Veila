use std::{path::Path, sync::Arc};

use super::{
    BackgroundAsset, BackgroundKind, BackgroundTreatment, GeneratedBackground, RenderCacheSummary,
    SourceCacheStatus,
    asset::{decode_rgba_image, unique_sizes},
    load_cached_generated_render, load_cached_render,
    source_cache::{has_cached_rgba, store_cached_rgba},
    store_cached_generated_render, store_cached_render,
};
use crate::{ClearColor, FrameSize, Result, SoftwareBuffer, cache::file_cache_key};

/// Shares one lazily loaded wallpaper asset across a single prewarm job.
pub struct FileBackgroundPrewarm<'a> {
    path: &'a Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    asset: Option<(String, BackgroundAsset)>,
}

impl<'a> FileBackgroundPrewarm<'a> {
    pub fn new(path: &'a Path, fallback: ClearColor, treatment: BackgroundTreatment) -> Self {
        Self {
            path,
            fallback,
            treatment,
            asset: None,
        }
    }

    pub fn path(&self) -> &'a Path {
        self.path
    }

    pub fn treatment(&self) -> BackgroundTreatment {
        self.treatment
    }

    pub fn prewarm_source(&mut self) -> Result<SourceCacheStatus> {
        if has_cached_rgba(self.path) {
            return Ok(SourceCacheStatus::Hit);
        }

        let key = file_cache_key(self.path)?;
        let image = decode_rgba_image(self.path)?;
        store_cached_rgba(self.path, &image)?;
        // Keep the cold decode until this job finishes instead of reading its cache back.
        self.asset = Some((
            key,
            BackgroundAsset {
                kind: BackgroundKind::Image {
                    image: Arc::new(image),
                    fallback: self.fallback,
                },
                treatment: self.treatment,
            },
        ));
        Ok(SourceCacheStatus::Warmed)
    }

    pub fn render(&mut self, size: FrameSize) -> Result<SoftwareBuffer> {
        let key = file_cache_key(self.path)?;
        // A wallpaper replaced between stages must not reuse the previous source pixels.
        if let Some((loaded_key, asset)) = &self.asset
            && loaded_key == &key
        {
            return asset.render(size);
        }
        let asset = BackgroundAsset::load(Some(self.path), self.fallback, None, self.treatment)?;
        let buffer = asset.render(size);
        self.asset = Some((key, asset));
        buffer
    }

    pub fn prewarm_rendered(&mut self, sizes: &[FrameSize]) -> Result<RenderCacheSummary> {
        let mut cache_hits = 0;
        let mut warmed_sizes = 0;
        for size in unique_sizes(sizes) {
            if load_cached_render(self.path, size, self.treatment)?.is_some() {
                cache_hits += 1;
                continue;
            }

            let buffer = self.render(size)?;
            store_cached_render(self.path, size, self.treatment, &buffer)?;
            warmed_sizes += 1;
        }
        Ok(RenderCacheSummary {
            cache_hits,
            warmed_sizes,
        })
    }
}

pub fn prewarm_source(path: &Path) -> Result<SourceCacheStatus> {
    FileBackgroundPrewarm::new(
        path,
        ClearColor::opaque(0, 0, 0),
        BackgroundTreatment::default(),
    )
    .prewarm_source()
}

pub fn prewarm_rendered(
    path: &Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    sizes: &[FrameSize],
) -> Result<RenderCacheSummary> {
    FileBackgroundPrewarm::new(path, fallback, treatment).prewarm_rendered(sizes)
}

pub fn prewarm_rendered_generated(
    generated: GeneratedBackground,
    treatment: BackgroundTreatment,
    sizes: &[FrameSize],
) -> Result<RenderCacheSummary> {
    let unique_sizes = unique_sizes(sizes);
    let mut cache_hits = 0;
    let mut missing_sizes = Vec::new();

    for size in unique_sizes {
        if load_cached_generated_render(generated, size, treatment)?.is_some() {
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

    let asset = BackgroundAsset {
        kind: BackgroundKind::Generated(generated),
        treatment,
    };
    for size in &missing_sizes {
        let buffer = asset.render(*size)?;
        store_cached_generated_render(generated, *size, treatment, &buffer)?;
    }

    Ok(RenderCacheSummary {
        cache_hits,
        warmed_sizes: missing_sizes.len(),
    })
}

#[cfg(test)]
mod tests;
