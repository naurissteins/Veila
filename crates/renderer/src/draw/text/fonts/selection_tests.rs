use super::super::query::{escape_family, fontconfig_weight, parse_match, query};
use super::*;
use std::{
    fs,
    os::unix::ffi::OsStringExt,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "veila-font-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn fontconfig_framing_keeps_native_paths_and_collection_indices() {
    let (path, index) = parse_match(
        b"/fonts/\xff.ttc\x1f"
            .iter()
            .copied()
            .chain(b"65537\x1f".iter().copied())
            .collect::<Vec<_>>()
            .as_slice(),
    )
    .unwrap();
    assert_eq!(path.into_os_string().into_vec(), b"/fonts/\xff.ttc");
    assert_eq!(index & 0xffff, 1);
    for bad in [
        b"relative\x1f0\x1f".as_slice(),
        b"/font\x1fbad\x1f",
        b"/font\x1f0\x1fextra",
        b"/font\x1f0",
    ] {
        assert!(parse_match(bad).is_none(), "{bad:?}");
    }
}

#[test]
fn lookup_timeout_kills_and_reaps_the_child() {
    let mut command = Command::new("sh");
    command.args(["-c", "exec sleep 10"]);
    let started = Instant::now();
    assert!(query(&mut command, "unused", started + Duration::from_millis(100)).is_none());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn lookup_output_and_failure_are_bounded() {
    for script in ["head -c 10000 /dev/zero", "exit 1", "printf garbage"] {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        assert!(
            query(
                &mut command,
                "unused",
                Instant::now() + Duration::from_secs(1)
            )
            .is_none()
        );
    }
}

#[test]
fn files_reject_missing_special_oversized_and_absurd_collections() {
    let fixture = Fixture::new();
    assert!(read_font(&fixture.0.join("missing"), MAX_TOTAL_BYTES).is_none());
    assert!(read_font(&fixture.0, MAX_TOTAL_BYTES).is_none());
    let fifo = fixture.0.join("fifo");
    nix::unistd::mkfifo(&fifo, nix::sys::stat::Mode::S_IRUSR).unwrap();
    assert!(read_font(&fifo, MAX_TOTAL_BYTES).is_none());
    let oversized = fixture.0.join("oversized");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    assert!(read_font(&oversized, MAX_TOTAL_BYTES).is_none());
    let collection = fixture.0.join("collection");
    fs::write(&collection, b"ttcf\0\0\0\0\xff\xff\xff\xff").unwrap();
    assert!(read_font(&collection, MAX_TOTAL_BYTES).is_none());
    let font = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/Geom-SemiBold.ttf");
    assert!(read_font(&font, 1).is_none());
    assert!(read_font(&font, MAX_TOTAL_BYTES).is_some());
}

#[test]
fn family_patterns_escape_fontconfig_syntax_and_map_css_weights() {
    assert_eq!(
        escape_family("Name,with:options=\\"),
        "Name\\,with\\:options\\=\\\\"
    );
    for (weight, expected) in [
        (100, 0),
        (400, 80),
        (500, 100),
        (600, 180),
        (700, 200),
        (800, 205),
        (900, 210),
    ] {
        assert_eq!(fontconfig_weight(weight), expected);
    }
}

#[test]
fn successful_query_requires_complete_framing_and_exit_success() {
    let mut command = Command::new("sh");
    command.args(["-c", "printf '/font\\0370\\037'"]);
    assert_eq!(
        query(
            &mut command,
            "unused",
            Instant::now() + Duration::from_secs(1)
        ),
        Some((PathBuf::from("/font"), 0))
    );
}

#[test]
fn cached_matches_rebuild_the_same_font_context_until_the_file_or_request_changes() {
    let fixture = Fixture::new();
    let manifest = fixture.0.join("selection.json");
    let font = fixture
        .0
        .join(std::ffi::OsString::from_vec(b"clock-\xff.ttf".to_vec()));
    let bytes = include_bytes!("../../../../../../assets/fonts/Geom-SemiBold.ttf");
    fs::write(&font, bytes).unwrap();
    let patterns = [(String::from("serif:weight=180:slant=0"), Some("serif"))];
    let key = cache::key(&patterns);
    let mut selection = Selection::new();
    let stamp = selection
        .add(&patterns[0].0, Some("serif"), &font, 0, None)
        .unwrap()
        .clone();
    let font_key = selection.hash.finish();
    cache::write(
        &manifest,
        key,
        font_key,
        vec![Some(FontMatch::new(&font, 0, stamp))],
    )
    .unwrap();

    let saved = cache::read(&manifest, key, patterns.len()).unwrap();
    let cached = replay_selection(&saved, &patterns).unwrap();
    assert_eq!(saved.font_key, font_key);
    assert!(prepare_configured_fonts(&saved, &patterns).is_some());
    assert_eq!(cached.aliases, selection.aliases);
    let edited = [(String::from("serif:weight=200:slant=0"), Some("serif"))];
    assert!(cache::read(&manifest, cache::key(&edited), edited.len()).is_none());

    let replacement = fixture.0.join("replacement.ttf");
    fs::write(&replacement, bytes).unwrap();
    fs::rename(replacement, font).unwrap();
    assert!(prepare_configured_fonts(&saved, &patterns).is_none());
    assert!(replay_selection(&saved, &patterns).is_none());
}

#[test]
fn corrupted_selection_is_a_cache_miss() {
    let fixture = Fixture::new();
    let manifest = fixture.0.join("selection.json");
    fs::write(&manifest, b"{invalid").unwrap();
    assert!(cache::read(&manifest, 0, 0).is_none());
}
