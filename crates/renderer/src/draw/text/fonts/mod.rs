mod helper;
mod selection;
#[cfg(test)]
mod tests;
mod worker;

use super::context::{FONT_CONTEXT, FontContext};
use cosmic_text::fontdb::{Database, Source};
use std::{collections::hash_map::DefaultHasher, hash::Hasher, sync::Arc};

pub use helper::{FONT_QUERY_SUBCOMMAND, run_font_query};
pub use worker::FontLoader;

pub const MAX_CONFIGURED_FONT_REQUESTS: usize = 32;
const BUNDLED_FONTS: [&[u8]; 3] = [
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/fonts/Geom-SemiBold.ttf"
    )),
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/fonts/GoogleSansFlex_72pt-Regular.ttf"
    )),
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/fonts/Nunito-ExtraBoldItalic.ttf"
    )),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontRequest {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
}

#[derive(Debug)]
pub struct PreparedFonts(pub(super) FontContext);

/// a helper executable must dispatch `FONT_QUERY_SUBCOMMAND`; `None` runs Fontconfig directly
pub fn prepare_fonts(requests: &[FontRequest], helper: Option<&std::path::Path>) -> PreparedFonts {
    selection::prepare(requests, helper)
}

pub fn install_fonts(fonts: PreparedFonts) -> bool {
    let changed = FONT_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        if context.cache_key == fonts.0.cache_key {
            return false;
        }
        *context = fonts.0;
        true
    });
    if changed {
        super::raster::clear_text_raster_cache();
    }
    changed
}

pub fn font_cache_key() -> u64 {
    FONT_CONTEXT.with(|context| context.borrow().cache_key)
}

pub(super) fn bundled_database() -> (Database, DefaultHasher) {
    let mut db = Database::new();
    let mut hash = DefaultHasher::new();
    hash.write(b"veila-fonts-v1");
    for font in BUNDLED_FONTS {
        hash.write(font);
        db.load_font_source(Source::Binary(Arc::new(font)));
    }
    // A missing optional face must still leave every generic family usable.
    db.set_sans_serif_family("Geom");
    db.set_serif_family("Geom");
    db.set_monospace_family("Geom");
    (db, hash)
}

pub(super) fn bundled_fonts() -> PreparedFonts {
    let (db, hash) = bundled_database();
    PreparedFonts(FontContext::from_database(db, hash.finish()))
}
