use super::super::context::{FontContext, resolve_font_family_in_db};
use super::{FontRequest, MAX_CONFIGURED_FONT_REQUESTS, PreparedFonts, bundled_database};
use cosmic_text::fontdb::{Database, Source};
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::OpenOptions,
    hash::Hasher,
    io::{ErrorKind, Read},
    os::unix::{ffi::OsStringExt, fs::OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

const QUERY_TIMEOUT: Duration = Duration::from_millis(1500);
const LOAD_BUDGET: Duration = Duration::from_secs(3);
const MAX_QUERY_BYTES: usize = 4096;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_FACES: usize = 64;
// installed script faces remain available to cosmic-text's shaping and fallback engine
const FALLBACKS: [&str; 21] = [
    "Open Sans",
    "Noto Sans",
    "DejaVu Sans",
    "FreeSans",
    "Noto Sans Mono",
    "DejaVu Sans Mono",
    "FreeMono",
    "Noto Sans Symbols",
    "Noto Sans Symbols2",
    "Noto Color Emoji",
    "DejaVu Serif",
    "sans-serif:charset=0627",
    "sans-serif:charset=05d0",
    "sans-serif:charset=4e2d",
    "sans-serif:charset=3042",
    "sans-serif:charset=ac00",
    "sans-serif:charset=0915",
    "sans-serif:charset=09ac",
    "sans-serif:charset=0e01",
    "emoji:charset=1f600",
    "sans-serif:charset=f1105",
];

pub(super) fn prepare(requests: &[FontRequest], helper: Option<&Path>) -> PreparedFonts {
    let started = Instant::now();
    let deadline = started + LOAD_BUDGET;
    let (mut db, mut hash) = bundled_database();
    let bundled = db.clone();
    let mut aliases = HashMap::new();
    let mut loaded = HashMap::new();
    let mut remaining = MAX_TOTAL_BYTES;
    for pattern in FALLBACKS {
        let _ = load_pattern(
            &mut db,
            &mut loaded,
            &mut remaining,
            &mut hash,
            pattern,
            deadline,
            helper,
        );
    }
    // keep cosmic-text's existing generic-family and ordered fallback policy
    db.set_sans_serif_family("Open Sans");
    db.set_monospace_family("Noto Sans Mono");
    db.set_serif_family("DejaVu Serif");
    let mut styles = Vec::new();
    for request in requests.iter().take(MAX_CONFIGURED_FONT_REQUESTS) {
        let style = (request.weight, request.italic);
        if !styles.contains(&style) {
            styles.push(style);
            for family in ["Open Sans", "Noto Sans", "DejaVu Sans", "Noto Sans Mono"] {
                let pattern = format!(
                    "{family}:weight={}:slant={}",
                    fontconfig_weight(style.0),
                    if style.1 { 100 } else { 0 }
                );
                let _ = load_pattern(
                    &mut db,
                    &mut loaded,
                    &mut remaining,
                    &mut hash,
                    &pattern,
                    deadline,
                    helper,
                );
            }
        }
    }
    for request in requests.iter().take(MAX_CONFIGURED_FONT_REQUESTS) {
        let family = request.family.trim();
        if family.is_empty() || family.len() > 256 {
            continue;
        }
        if resolve_font_family_in_db(&bundled, family).is_some() {
            continue;
        }
        let pattern = format!(
            "{}:weight={}:slant={}",
            escape_family(family),
            fontconfig_weight(request.weight),
            if request.italic { 100 } else { 0 }
        );
        if let Some(resolved) = load_pattern(
            &mut db,
            &mut loaded,
            &mut remaining,
            &mut hash,
            &pattern,
            deadline,
            helper,
        ) {
            // fontconfig also returns a default for unknown names; keep the existing UI fallback
            if matches!(
                family.to_ascii_lowercase().as_str(),
                "sans-serif" | "sans" | "serif" | "monospace" | "system-ui" | "ui-monospace"
            ) {
                aliases.insert(family.to_owned(), Some(resolved));
            }
        }
    }
    if loaded.is_empty() {
        tracing::warn!(
            "optional system fonts unavailable; using bundled fonts (check fc-match and installed fonts)"
        );
    }
    tracing::info!(
        faces = db.faces().count(),
        bytes = MAX_TOTAL_BYTES - remaining,
        elapsed_ms = started.elapsed().as_millis(),
        "bounded fonts prepared"
    );
    let mut context = FontContext::from_database(db, hash.finish());
    context.resolved_families = aliases;
    // materialize selected faces here so first use performs no lazy font-file parsing in dispatch
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

fn load_pattern(
    db: &mut Database,
    loaded: &mut HashMap<PathBuf, Vec<cosmic_text::fontdb::ID>>,
    remaining: &mut u64,
    hash: &mut impl Hasher,
    pattern: &str,
    deadline: Instant,
    helper: Option<&Path>,
) -> Option<String> {
    if Instant::now() >= deadline || db.faces().count() >= MAX_FACES {
        return None;
    }
    let mut command = Command::new(helper.unwrap_or_else(|| Path::new("fc-match")));
    if helper.is_some() {
        command
            .arg(super::helper::FONT_QUERY_SUBCOMMAND)
            .env(super::helper::PARENT_ENV, std::process::id().to_string());
    }
    let (path, index) = query(&mut command, pattern, deadline)?;
    if !loaded.contains_key(&path) {
        let bytes = read_font(&path, *remaining)?;
        let length = bytes.len() as u64;
        let bytes = Arc::new(bytes);
        let ids = db.load_font_source(Source::Binary(bytes.clone())).to_vec();
        if ids.is_empty() {
            return None;
        }
        if db.faces().count() > MAX_FACES {
            for id in ids {
                db.remove_face(id);
            }
            return None;
        }
        *remaining -= length;
        hash.write(&bytes);
        loaded.insert(path.clone(), ids);
    }
    hash.write_usize(pattern.len());
    hash.write(pattern.as_bytes());
    hash.write_u32(index);
    loaded
        .get(&path)?
        .iter()
        .filter_map(|id| db.face(*id))
        .find(|face| face.index == index & 0xffff)?
        .families
        .first()
        .map(|(family, _)| family.clone())
}

fn read_font(path: &Path, remaining: u64) -> Option<Vec<u8>> {
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
    let mut data = Vec::new();
    file.take(limit + 1).read_to_end(&mut data).ok()?;
    if data.len() as u64 > limit {
        return None;
    }
    // reject absurd collection counts before fontdb reserves one entry per declared face
    if data.starts_with(b"ttcf") {
        let count = u32::from_be_bytes(data.get(8..12)?.try_into().ok()?);
        if count == 0 || count as usize > MAX_FACES {
            return None;
        }
    }
    Some(data)
}

fn query(command: &mut Command, pattern: &str, deadline: Instant) -> Option<(PathBuf, u32)> {
    let deadline = deadline.min(Instant::now() + QUERY_TIMEOUT);
    let mut child = command
        .args(["-f", super::helper::QUERY_FORMAT, "--", pattern])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let result = (|| {
        let mut stdout = child.stdout.take()?;
        fcntl(&stdout, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).ok()?;
        let mut output = Vec::new();
        loop {
            let mut buffer = [0; 512];
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    if let Some(status) = child.try_wait().ok()? {
                        return status.success().then(|| parse_match(&output)).flatten();
                    }
                }
                Ok(length) => {
                    if output.len() + length > MAX_QUERY_BYTES {
                        return None;
                    }
                    output.extend_from_slice(&buffer[..length]);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => (),
                Err(_) => return None,
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(2));
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn parse_match(output: &[u8]) -> Option<(PathBuf, u32)> {
    let mut fields = output.split(|byte| *byte == 0x1f);
    let path = PathBuf::from(OsString::from_vec(fields.next()?.to_vec()));
    let index = std::str::from_utf8(fields.next()?).ok()?.parse().ok()?;
    if !path.is_absolute() || fields.next()? != b"" || fields.next().is_some() {
        return None;
    }
    Some((path, index))
}

fn escape_family(family: &str) -> String {
    let mut escaped = String::new();
    for ch in family.chars() {
        if matches!(ch, '\\' | ':' | ',' | '=') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn fontconfig_weight(weight: u16) -> u16 {
    match weight {
        0..=149 => 0,
        150..=249 => 40,
        250..=349 => 50,
        350..=449 => 80,
        450..=549 => 100,
        550..=649 => 180,
        650..=749 => 200,
        750..=849 => 205,
        _ => 210,
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
