use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Mutex, OnceLock, mpsc},
    thread::{self, JoinHandle},
    thread_local,
    time::Duration,
};

use cosmic_text::{
    FontSystem, SwashCache,
    fontdb::{Database, Source},
};

const BUNDLED_CLOCK_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/Geom-SemiBold.ttf"
));
const BUNDLED_GOOGLE_SANS_FLEX_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/GoogleSansFlex_72pt-Regular.ttf"
));
const BUNDLED_NUNITO_EXTRABOLD_ITALIC_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/Nunito-ExtraBoldItalic.ttf"
));
const BUNDLED_FONTS: [&[u8]; 3] = [
    BUNDLED_CLOCK_FONT,
    BUNDLED_GOOGLE_SANS_FLEX_FONT,
    BUNDLED_NUNITO_EXTRABOLD_ITALIC_FONT,
];

struct FontWarmupTask {
    handle: JoinHandle<FontContext>,
    families: mpsc::Sender<Vec<String>>,
}

static FONT_WARMUP: OnceLock<Mutex<Option<FontWarmupTask>>> = OnceLock::new();

#[derive(Debug)]
pub(super) struct FontContext {
    pub(super) font_system: FontSystem,
    pub(super) swash_cache: SwashCache,
    resolved_families: HashMap<String, Option<String>>,
}

thread_local! {
    pub(super) static FONT_CONTEXT: RefCell<FontContext> = RefCell::new(take_warmed_context());
}

pub fn start_font_warmup() {
    let warmup = FONT_WARMUP.get_or_init(|| Mutex::new(None));
    let Ok(mut warmup) = warmup.lock() else {
        return;
    };
    if warmup.is_some() {
        return;
    }

    let (sender, receiver) = mpsc::channel::<Vec<String>>();
    if let Ok(handle) = thread::Builder::new()
        .name(String::from("veila-font-warmup"))
        .spawn(move || {
            let mut context = FontContext::new();
            let families = receiver
                .recv_timeout(Duration::from_millis(20))
                .unwrap_or_default();
            context.resolve_families(&families);
            context
        })
    {
        *warmup = Some(FontWarmupTask {
            handle,
            families: sender,
        });
    }
}

pub fn configure_font_warmup(families: Vec<String>) {
    if let Some(warmup) = FONT_WARMUP.get()
        && let Ok(warmup) = warmup.lock()
        && let Some(task) = warmup.as_ref()
    {
        let _ = task.families.send(families);
    }
}

fn take_warmed_context() -> FontContext {
    let task = FONT_WARMUP
        .get()
        .and_then(|warmup| warmup.lock().ok()?.take());
    task.and_then(|task| task.handle.join().ok())
        .unwrap_or_else(FontContext::new)
}

impl FontContext {
    fn new() -> Self {
        Self {
            font_system: font_system_with_system_and_bundled_fonts(),
            swash_cache: SwashCache::new(),
            resolved_families: HashMap::new(),
        }
    }

    fn resolve_families(&mut self, configured_families: &[String]) {
        for requested in configured_families {
            let resolved = resolve_font_family_in_db(self.font_system.db(), requested);
            self.resolved_families.insert(requested.clone(), resolved);
        }
    }
}

fn font_system_with_system_and_bundled_fonts() -> FontSystem {
    let mut font_system = FontSystem::new();
    for font in BUNDLED_FONTS {
        font_system
            .db_mut()
            .load_font_source(Source::Binary(std::sync::Arc::new(font)));
    }
    font_system
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
