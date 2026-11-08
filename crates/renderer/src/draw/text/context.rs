use cosmic_text::{FontSystem, SwashCache, fontdb::Database};
use std::{cell::RefCell, collections::HashMap, thread_local};

#[derive(Debug)]
pub(super) struct FontContext {
    pub(super) font_system: FontSystem,
    pub(super) swash_cache: SwashCache,
    pub(super) resolved_families: HashMap<String, Option<String>>,
    pub(super) cache_key: u64,
}

thread_local! {
    pub(super) static FONT_CONTEXT: RefCell<FontContext> = RefCell::new(super::fonts::bundled_fonts().0);
}

impl FontContext {
    pub(super) fn from_database(db: Database, cache_key: u64) -> Self {
        let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
        Self {
            font_system: FontSystem::new_with_locale_and_db(locale, db),
            swash_cache: SwashCache::new(),
            resolved_families: HashMap::new(),
            cache_key,
        }
    }
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
        let mut context = context.borrow_mut();
        if let Some(resolved) = context.resolved_families.get(requested) {
            return resolved.clone();
        }
        let resolved = resolve_font_family_in_db(context.font_system.db(), requested);
        context
            .resolved_families
            .insert(requested.to_owned(), resolved.clone());
        resolved
    })
}

pub(super) fn resolve_font_family_in_db(db: &Database, requested: &str) -> Option<String> {
    let requested = normalize_font_name(requested);
    if requested.is_empty() {
        return None;
    }
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

pub(super) fn normalize_font_name(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
