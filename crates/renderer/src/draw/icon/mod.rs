mod parser;
mod raster;

#[cfg(test)]
mod tests;

use std::{cell::RefCell, thread_local};

use crate::{ClearColor, SoftwareBuffer, shape::Rect};

use parser::{ParsedIcon, eye_icon, eye_off_icon, user_icon};
use raster::{blend_icon_raster, rasterize_icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetIcon {
    Eye,
    EyeOff,
    User,
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

    let width = rect.width.max(1) as u32;
    let height = rect.height.max(1) as u32;
    let key = IconRasterKey {
        icon,
        width,
        height,
        color: style.color,
        padding: style.padding,
    };

    ICON_RASTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache
            .iter()
            .position(|entry| entry.key == key)
            .unwrap_or_else(|| {
                cache.push(CachedRasterIcon {
                    key,
                    pixels: rasterize_icon(key, parsed_icon(key.icon)),
                });
                cache.len() - 1
            });
        let raster = &cache[index];
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

fn parsed_icon(icon: AssetIcon) -> &'static ParsedIcon {
    match icon {
        AssetIcon::Eye => eye_icon(),
        AssetIcon::EyeOff => eye_off_icon(),
        AssetIcon::User => user_icon(),
    }
}
