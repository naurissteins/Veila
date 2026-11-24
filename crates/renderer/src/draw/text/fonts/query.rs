use super::super::context::resolve_font_family_in_db;
use super::{FontRequest, MAX_CONFIGURED_FONT_REQUESTS};
use cosmic_text::fontdb::Database;
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use std::{
    ffi::OsString,
    io::{ErrorKind, Read},
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const QUERY_TIMEOUT: Duration = Duration::from_millis(1500);
const MAX_QUERY_BYTES: usize = 4096;

pub(super) fn query_pattern(
    pattern: &str,
    helper: Option<&Path>,
    deadline: Instant,
) -> Option<(PathBuf, u32)> {
    if Instant::now() >= deadline {
        return None;
    }
    let mut command = Command::new(helper.unwrap_or_else(|| Path::new("fc-match")));
    if helper.is_some() {
        command
            .arg(super::helper::FONT_QUERY_SUBCOMMAND)
            .env(super::helper::PARENT_ENV, std::process::id().to_string());
    }
    query(&mut command, pattern, deadline)
}

pub(super) fn query(
    command: &mut Command,
    pattern: &str,
    deadline: Instant,
) -> Option<(PathBuf, u32)> {
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

pub(super) fn parse_match(output: &[u8]) -> Option<(PathBuf, u32)> {
    let mut fields = output.split(|byte| *byte == 0x1f);
    let path = PathBuf::from(OsString::from_vec(fields.next()?.to_vec()));
    let index = std::str::from_utf8(fields.next()?).ok()?.parse().ok()?;
    if !path.is_absolute() || fields.next()? != b"" || fields.next().is_some() {
        return None;
    }
    Some((path, index))
}
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

pub(super) fn font_patterns<'a>(
    requests: &'a [FontRequest],
    bundled: &Database,
) -> Vec<(String, Option<&'a str>)> {
    let requests = requests.iter().take(MAX_CONFIGURED_FONT_REQUESTS);
    let mut patterns: Vec<_> = FALLBACKS
        .iter()
        .map(|pattern| ((*pattern).to_owned(), None))
        .collect();
    let mut styles = Vec::new();
    for request in requests.clone() {
        let style = (request.weight, request.italic);
        if !styles.contains(&style) {
            styles.push(style);
            for family in ["Open Sans", "Noto Sans", "DejaVu Sans", "Noto Sans Mono"] {
                patterns.push((font_pattern(family, request), None));
            }
        }
    }
    for request in requests {
        let family = request.family.trim();
        if !family.is_empty()
            && family.len() <= 256
            && resolve_font_family_in_db(bundled, family).is_none()
        {
            patterns.push((font_pattern(family, request), Some(family)));
        }
    }
    patterns
}

fn font_pattern(family: &str, request: &FontRequest) -> String {
    format!(
        "{}:weight={}:slant={}",
        escape_family(family),
        fontconfig_weight(request.weight),
        if request.italic { 100 } else { 0 },
    )
}

pub(super) fn escape_family(family: &str) -> String {
    let mut escaped = String::new();
    for ch in family.chars() {
        if matches!(ch, '\\' | ':' | ',' | '=') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

pub(super) fn fontconfig_weight(weight: u16) -> u16 {
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
