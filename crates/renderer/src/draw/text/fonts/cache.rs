use std::{
    fs::{Metadata, OpenOptions},
    hash::{DefaultHasher, Hash, Hasher},
    io::{self, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

const MAX_BYTES: u64 = 512 * 1024;
const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct FileStamp {
    device: u64,
    inode: u64,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl From<&Metadata> for FileStamp {
    fn from(meta: &Metadata) -> Self {
        Self {
            device: meta.dev(),
            inode: meta.ino(),
            length: meta.len(),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct FontMatch {
    path: Vec<u8>,
    pub index: u32,
    pub stamp: FileStamp,
}

impl FontMatch {
    pub fn new(path: &Path, index: u32, stamp: FileStamp) -> Self {
        Self {
            path: path.as_os_str().as_bytes().to_vec(),
            index,
            stamp,
        }
    }

    pub fn path(&self) -> &Path {
        Path::new(std::ffi::OsStr::from_bytes(&self.path))
    }

    pub fn byte_len(&self) -> u64 {
        self.stamp.length
    }

    pub fn is_current(&self) -> bool {
        self.path().is_absolute()
            && std::fs::metadata(self.path())
                .is_ok_and(|meta| meta.is_file() && FileStamp::from(&meta) == self.stamp)
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct Manifest {
    key: u64,
    pub font_key: u64,
    pub matches: Vec<Option<FontMatch>>,
}

pub(super) fn path() -> Option<PathBuf> {
    Some(crate::cache::resolve_cache_root(None)?.join("fonts/selection.json"))
}

pub(super) fn key(patterns: &[(String, Option<&str>)]) -> u64 {
    let mut hash = DefaultHasher::new();
    (
        "veila-font-selection-v2",
        env!("CARGO_PKG_VERSION"),
        patterns,
    )
        .hash(&mut hash);
    for key in [
        "FONTCONFIG_FILE",
        "FONTCONFIG_PATH",
        "FONTCONFIG_SYSROOT",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_DATA_DIRS",
        "HOME",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "FC_LANG",
    ] {
        std::env::var_os(key).hash(&mut hash);
    }
    hash.finish()
}

pub(super) fn read(path: &Path, key: u64, pattern_count: usize) -> Option<Manifest> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file()
        || metadata.len() > MAX_BYTES
        || metadata.modified().ok()?.elapsed().ok()? > MAX_AGE
    {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > MAX_BYTES {
        return None;
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    (manifest.key == key && manifest.matches.len() == pattern_count).then_some(manifest)
}

pub(super) fn write(
    path: &Path,
    key: u64,
    font_key: u64,
    matches: Vec<Option<FontMatch>>,
) -> io::Result<()> {
    let bytes = serde_json::to_vec(&Manifest {
        key,
        font_key,
        matches,
    })?;
    if bytes.len() as u64 > MAX_BYTES {
        return Ok(());
    }
    crate::cache::write_atomic(path, |file| std::io::Write::write_all(file, &bytes))
}
