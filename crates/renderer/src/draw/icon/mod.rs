mod parser;
mod raster;
mod weather;

#[cfg(test)]
mod tests;

use std::{cell::RefCell, thread_local};

use crate::{ClearColor, SoftwareBuffer, shape::Rect};

use parser::{ParsedIcon, eye_icon, eye_off_icon, user_icon};
use raster::{blend_icon_raster, rasterize_icon, visible_alpha_bounds};
pub use weather::WeatherIcon;
use weather::weather_svg;

pub(super) enum IconRasterSource {
    Parsed(&'static ParsedIcon),
    Svg(&'static [u8]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetIcon {
    Eye,
    EyeOff,
    User,
    Weather(WeatherIcon),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconStyle {
    pub color: ClearColor,
    pub padding: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedRasterIcon {
    key: IconRasterKey,
    pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IconRasterKey {
    pub(super) icon: AssetIcon,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) color: ClearColor,
    pub(super) padding: i32,
}

thread_local! {
    pub(super) static ICON_RASTER_CACHE: RefCell<Vec<CachedRasterIcon>> = const { RefCell::new(Vec::new()) };
}

impl IconStyle {
    pub const fn new(color: ClearColor) -> Self {
        Self { color, padding: 3 }
    }

    pub const fn with_padding(self, padding: i32) -> Self {
        Self {
            color: self.color,
            padding,
        }
    }
}

pub fn draw_icon(buffer: &mut SoftwareBuffer, rect: Rect, icon: AssetIcon, style: IconStyle) {
    if rect.is_empty() {
        return;
    }

    let key = icon_raster_key(rect, icon, style);
    with_cached_icon_raster(key, |raster| {
        blend_icon_raster(
            buffer,
            rect.x,
            rect.y,
            raster.key.width,
            raster.key.height,
            &raster.pixels,
        );
    });
}

pub fn icon_visible_bounds(rect: Rect, icon: AssetIcon, style: IconStyle) -> Option<Rect> {
    if rect.is_empty() {
        return None;
    }

    let key = icon_raster_key(rect, icon, style);
    with_cached_icon_raster(key, |raster| {
        let bounds = visible_alpha_bounds(&raster.pixels, key.width, key.height)?;
        Some(Rect::new(
            rect.x + bounds.left as i32,
            rect.y + bounds.top as i32,
            bounds.width() as i32,
            bounds.height() as i32,
        ))
    })
}

fn icon_raster_key(rect: Rect, icon: AssetIcon, style: IconStyle) -> IconRasterKey {
    IconRasterKey {
        icon,
        width: rect.width.max(1) as u32,
        height: rect.height.max(1) as u32,
        color: style.color,
        padding: style.padding,
    }
}

fn with_cached_icon_raster<T>(key: IconRasterKey, f: impl FnOnce(&CachedRasterIcon) -> T) -> T {
    ICON_RASTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache
            .iter()
            .position(|entry| entry.key == key)
            .unwrap_or_else(|| {
                cache.push(CachedRasterIcon {
                    key,
                    pixels: rasterize_icon(key, icon_source(key.icon)),
                });
                cache.len() - 1
            });
        f(&cache[index])
    })
}

fn icon_source(icon: AssetIcon) -> IconRasterSource {
    match icon {
        AssetIcon::Eye => IconRasterSource::Parsed(eye_icon()),
        AssetIcon::EyeOff => IconRasterSource::Parsed(eye_off_icon()),
        AssetIcon::User => IconRasterSource::Parsed(user_icon()),
        AssetIcon::Weather(icon) => IconRasterSource::Svg(weather_svg(icon)),
    }
}
