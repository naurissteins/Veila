mod asset;
mod render;
mod render_cache;
mod source_cache;
#[cfg(test)]
mod tests;
mod treatment;

use std::sync::Arc;

use image::RgbaImage;

use crate::ClearColor;

pub use asset::{
    load_cached_render, load_cached_render_variant, prewarm_rendered, prewarm_source,
    store_cached_render, store_cached_render_variant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCacheStatus {
    Hit,
    Warmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCacheSummary {
    pub cache_hits: usize,
    pub warmed_sizes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackgroundTreatment {
    pub blur_radius: u8,
    pub dim_strength: u8,
    pub tint: Option<ClearColor>,
    pub tint_opacity: u8,
}

#[derive(Debug, Clone)]
pub struct BackgroundAsset {
    kind: BackgroundKind,
    treatment: BackgroundTreatment,
}

#[derive(Debug, Clone)]
enum BackgroundKind {
    Solid(ClearColor),
    Image(Arc<RgbaImage>),
}
