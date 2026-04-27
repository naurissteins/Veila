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
    load_cached_generated_render, load_cached_generated_render_variant, load_cached_render,
    load_cached_render_variant, prewarm_rendered, prewarm_rendered_generated, prewarm_source,
    store_cached_generated_render, store_cached_generated_render_variant, store_cached_render,
    store_cached_render_variant,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundGradient {
    pub top_left: ClearColor,
    pub top_right: ClearColor,
    pub bottom_left: ClearColor,
    pub bottom_right: ClearColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundRadial {
    pub center: ClearColor,
    pub edge: ClearColor,
    pub center_x: u8,
    pub center_y: u8,
    pub radius: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundLayeredBlob {
    pub color: ClearColor,
    pub x: u8,
    pub y: u8,
    pub size: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundLayeredBase {
    Solid(ClearColor),
    Gradient(BackgroundGradient),
    Radial(BackgroundRadial),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundLayered {
    pub base: BackgroundLayeredBase,
    pub blobs: [Option<BackgroundLayeredBlob>; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedBackground {
    Gradient(BackgroundGradient),
    Layered(BackgroundLayered),
    Radial(BackgroundRadial),
}

impl GeneratedBackground {
    pub const fn mode_name(self) -> &'static str {
        match self {
            Self::Gradient(_) => "gradient",
            Self::Layered(_) => "layered",
            Self::Radial(_) => "radial",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackgroundAsset {
    kind: BackgroundKind,
    treatment: BackgroundTreatment,
}

#[derive(Debug, Clone)]
enum BackgroundKind {
    Solid(ClearColor),
    Generated(GeneratedBackground),
    Image(Arc<RgbaImage>),
}
