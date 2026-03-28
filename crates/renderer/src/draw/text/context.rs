use std::{cell::RefCell, thread_local};

use cosmic_text::{FontSystem, SwashCache, fontdb::Database};

const BUNDLED_CLOCK_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/Geom-SemiBold.ttf"
));
const BUNDLED_WEATHER_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/Geom-SemiBold.ttf"
));
const BUNDLED_GOOGLE_SANS_FLEX_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/GoogleSansFlex_72pt-Regular.ttf"
));
const BUNDLED_RALEWAY_SEMIBOLD_ITALIC_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/Raleway-SemiBoldItalic.ttf"
));
const BUNDLED_FONTS: [&[u8]; 4] = [
    BUNDLED_CLOCK_FONT,
    BUNDLED_WEATHER_FONT,
    BUNDLED_GOOGLE_SANS_FLEX_FONT,
    BUNDLED_RALEWAY_SEMIBOLD_ITALIC_FONT,
];

#[derive(Debug)]
pub(super) struct FontContext {
    pub(super) font_system: FontSystem,
    pub(super) swash_cache: SwashCache,
}

thread_local! {
    pub(super) static FONT_CONTEXT: RefCell<FontContext> = RefCell::new(FontContext {
        font_system: {
            let mut font_system = FontSystem::new();
            for font in BUNDLED_FONTS {
                font_system.db_mut().load_font_data(font.to_vec());
            }
            font_system
        },
        swash_cache: SwashCache::new(),
    });
}

pub fn bundled_clock_font_family() -> Option<String> {
    FONT_CONTEXT.with(|context| {
        let context = context.borrow();
        context
            .font_system
            .db()
            .faces()
            .find(|face| matches!(&face.source, cosmic_text::fontdb::Source::Binary(_)))
            .and_then(|face| face.families.first().map(|(family, _)| family.clone()))
    })
}

pub fn bundled_clock_font_postscript_name() -> Option<String> {
    FONT_CONTEXT.with(|context| {
        let context = context.borrow();
        context
            .font_system
            .db()
            .faces()
            .find(|face| matches!(&face.source, cosmic_text::fontdb::Source::Binary(_)))
            .map(|face| face.post_script_name.clone())
    })
}

pub fn resolve_font_family(requested: &str) -> Option<String> {
    let requested = requested.trim();
    if requested.is_empty() {
        return None;
    }

    FONT_CONTEXT.with(|context| {
        let context = context.borrow();
        resolve_font_family_in_db(context.font_system.db(), requested)
    })
}

fn resolve_font_family_in_db(db: &Database, requested: &str) -> Option<String> {
    let requested = normalize_font_name(requested);
    let mut partial_match = None;

    for face in db.faces() {
        for (family, _) in &face.families {
            let normalized_family = normalize_font_name(family);
            if normalized_family == requested {
                return Some(family.clone());
            }

            if partial_match.is_none()
                && (normalized_family.contains(&requested)
                    || requested.contains(&normalized_family))
            {
                partial_match = Some(family.clone());
            }
        }

        if normalize_font_name(&face.post_script_name) == requested
            && let Some((family, _)) = face.families.first()
        {
            return Some(family.clone());
        }
    }

    partial_match
}

fn normalize_font_name(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
