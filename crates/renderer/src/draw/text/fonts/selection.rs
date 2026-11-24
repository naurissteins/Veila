use super::super::context::{FontContext, resolve_font_family_in_db};
use super::{
    FontRequest, PreparedFonts, bundled_database,
    cache::{self, FileStamp, FontMatch},
    query::{font_patterns, query_pattern},
};
use cosmic_text::fontdb::{Database, Source};
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    fs::OpenOptions,
    hash::Hasher,
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

const LOAD_BUDGET: Duration = Duration::from_secs(3);
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_STARTUP_FONT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_FACES: usize = 64;
struct LoadedFile {
    faces: Vec<cosmic_text::fontdb::ID>,
    stamp: FileStamp,
}

struct Selection {
    db: Database,
    hash: DefaultHasher,
    loaded: HashMap<PathBuf, LoadedFile>,
    aliases: HashMap<String, Option<String>>,
    remaining: u64,
}

impl Selection {
    fn new() -> Self {
        let (db, hash) = bundled_database();
        Self {
            db,
            hash,
            loaded: HashMap::new(),
            aliases: HashMap::new(),
            remaining: MAX_TOTAL_BYTES,
        }
    }

    fn add(
        &mut self,
        pattern: &str,
        family: Option<&str>,
        path: &Path,
        index: u32,
        expected: Option<&FileStamp>,
    ) -> Option<&FileStamp> {
        if self.db.faces().count() >= MAX_FACES || !path.is_absolute() {
            return None;
        }
        if !self.loaded.contains_key(path) {
            let (bytes, stamp) = read_font(path, self.remaining)?;
            if expected.is_some_and(|expected| *expected != stamp) {
                return None;
            }
            let bytes = Arc::new(bytes);
            let faces = self
                .db
                .load_font_source(Source::Binary(bytes.clone()))
                .to_vec();
            if self.db.faces().count() > MAX_FACES || self.face_family(&faces, index).is_none() {
                for id in faces {
                    self.db.remove_face(id);
                }
                return None;
            }
            self.remaining -= bytes.len() as u64;
            if expected.is_none() {
                self.hash.write(&bytes);
            }
            self.loaded
                .insert(path.to_owned(), LoadedFile { faces, stamp });
        }
        let file = self.loaded.get(path)?;
        if expected.is_some_and(|expected| *expected != file.stamp) {
            return None;
        }
        let resolved = self.face_family(&file.faces, index)?;
        if let Some(family) = family.filter(|family| is_generic(family)) {
            self.aliases
                .insert(family.to_owned(), Some(resolved.to_owned()));
        }
        if expected.is_none() {
            self.hash.write_usize(pattern.len());
            self.hash.write(pattern.as_bytes());
            self.hash.write_u32(index);
        }
        Some(&file.stamp)
    }

    fn face_family(&self, faces: &[cosmic_text::fontdb::ID], index: u32) -> Option<&str> {
        faces
            .iter()
            .filter_map(|id| self.db.face(*id))
            .find(|face| face.index == index & 0xffff)?
            .families
            .first()
            .map(|(family, _)| family.as_str())
    }

    fn finish(self, font_key: u64) -> PreparedFonts {
        if self.loaded.is_empty() {
            tracing::warn!(
                "optional system fonts unavailable; using bundled fonts (check fc-match and installed fonts)"
            );
        }
        let mut context = FontContext::from_database(self.db, font_key);
        context.resolved_families = self.aliases;
        // Materialize selected faces off the dispatch thread.
        let faces: Vec<_> = context
            .font_system
            .db()
            .faces()
            .map(|face| (face.id, face.weight))
            .collect();
        for (id, weight) in faces {
            let _ = context.font_system.get_font(id, weight);
        }
        PreparedFonts(context)
    }
}

pub(super) fn prepare_cached(
    requests: &[FontRequest],
    helper: &Path,
    publish: &dyn Fn(PreparedFonts),
) -> PreparedFonts {
    let path = cache::path();
    prepare_with_cache(requests, Some(helper), path.as_deref(), publish)
}

pub(super) fn prepare(requests: &[FontRequest], helper: Option<&Path>) -> PreparedFonts {
    prepare_with_cache(requests, helper, None, &|_| {})
}

fn prepare_with_cache(
    requests: &[FontRequest],
    helper: Option<&Path>,
    cache_path: Option<&Path>,
    publish: &dyn Fn(PreparedFonts),
) -> PreparedFonts {
    let started = Instant::now();
    let mut selection = Selection::new();
    let patterns = font_patterns(requests, &selection.db);
    let key = cache::key(&patterns);
    if let Some(path) = cache_path
        && let Some(manifest) = cache::read(path, key, patterns.len())
        && manifest.matches.iter().flatten().all(FontMatch::is_current)
    {
        // Large system fallbacks can finish after the theme's faces are ready.
        if let Some(fonts) = prepare_configured_fonts(&manifest, &patterns) {
            tracing::debug!(
                elapsed_ms = started.elapsed().as_millis(),
                "prepared cached theme fonts"
            );
            publish(fonts);
        }
        if let Some(cached) = replay_selection(&manifest, &patterns) {
            let fonts = finish_selection(cached, manifest.font_key, started);
            tracing::debug!(
                elapsed_ms = started.elapsed().as_millis(),
                "reused cached font selection"
            );
            return fonts;
        }
    }
    let deadline = Instant::now() + LOAD_BUDGET;
    let mut matches = Vec::new();
    let mut complete = true;
    for (pattern, family) in &patterns {
        let found = (selection.db.faces().count() < MAX_FACES)
            .then(|| query_pattern(pattern, helper, deadline))
            .flatten()
            .and_then(|(path, index)| {
                let stamp = selection.add(pattern, *family, &path, index, None)?;
                Some(FontMatch::new(&path, index, stamp.clone()))
            });
        if let Some(name) = family {
            complete &= found.is_some()
                && (is_generic(name) || resolve_font_family_in_db(&selection.db, name).is_some());
        }
        matches.push(found);
    }
    let font_key = selection.hash.finish();
    if complete
        && !selection.loaded.is_empty()
        && Instant::now() < deadline
        && let Some(path) = cache_path
        && let Err(error) = cache::write(path, key, font_key, matches)
    {
        tracing::debug!(%error, "failed to save font selection");
    }
    finish_selection(selection, font_key, started)
}

fn finish_selection(mut selection: Selection, font_key: u64, started: Instant) -> PreparedFonts {
    // The startup subset keeps bundled generic families until the system fallbacks are ready.
    selection.db.set_sans_serif_family("Open Sans");
    selection.db.set_monospace_family("Noto Sans Mono");
    selection.db.set_serif_family("DejaVu Serif");
    let (faces, bytes) = (
        selection.db.faces().count(),
        MAX_TOTAL_BYTES - selection.remaining,
    );
    let fonts = selection.finish(font_key);
    tracing::info!(
        faces,
        bytes,
        elapsed_ms = started.elapsed().as_millis(),
        "bounded fonts prepared"
    );
    fonts
}

fn prepare_configured_fonts(
    manifest: &cache::Manifest,
    patterns: &[(String, Option<&str>)],
) -> Option<PreparedFonts> {
    let mut selection = Selection::new();
    selection.remaining = MAX_STARTUP_FONT_BYTES;
    for ((pattern, family), matched) in patterns.iter().zip(&manifest.matches) {
        let Some(family) = family else {
            continue;
        };
        let matched = matched.as_ref()?;
        if !selection.loaded.contains_key(matched.path())
            && matched.byte_len() > selection.remaining
        {
            return None;
        }
        selection.add(
            pattern,
            Some(family),
            matched.path(),
            matched.index,
            Some(&matched.stamp),
        )?;
        if !is_generic(family) && resolve_font_family_in_db(&selection.db, family).is_none() {
            return None;
        }
    }
    let mut hash = DefaultHasher::new();
    hash.write(b"veila-configured-startup-fonts-v1");
    hash.write_u64(manifest.font_key);
    (!selection.loaded.is_empty()).then(|| selection.finish(hash.finish()))
}

fn replay_selection(
    manifest: &cache::Manifest,
    patterns: &[(String, Option<&str>)],
) -> Option<Selection> {
    let mut selection = Selection::new();
    for ((pattern, family), matched) in patterns.iter().zip(&manifest.matches) {
        if let Some(matched) = matched {
            selection.add(
                pattern,
                *family,
                matched.path(),
                matched.index,
                Some(&matched.stamp),
            )?;
        } else if family.is_some() {
            return None;
        }
    }
    (!selection.loaded.is_empty()).then_some(selection)
}

fn is_generic(family: &str) -> bool {
    matches!(
        family.to_ascii_lowercase().as_str(),
        "sans-serif" | "sans" | "serif" | "monospace" | "system-ui" | "ui-monospace"
    )
}

fn read_font(path: &Path, remaining: u64) -> Option<(Vec<u8>, FileStamp)> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let metadata = file.metadata().ok()?;
    let limit = MAX_FILE_BYTES.min(remaining);
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > limit {
        return None;
    }
    let mut data = Vec::with_capacity(metadata.len() as usize);
    (&file).take(limit + 1).read_to_end(&mut data).ok()?;
    let stamp = FileStamp::from(&metadata);
    if data.len() as u64 != metadata.len() || FileStamp::from(&file.metadata().ok()?) != stamp {
        return None;
    }
    // reject absurd collection counts before fontdb reserves one entry per declared face
    if data.starts_with(b"ttcf") {
        let count = u32::from_be_bytes(data.get(8..12)?.try_into().ok()?);
        if count == 0 || count as usize > MAX_FACES {
            return None;
        }
    }
    Some((data, stamp))
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
