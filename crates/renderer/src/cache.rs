mod atomic;
pub(crate) mod image;
mod prune;
pub use prune::{CacheKind, CachePrunePolicy, CachePruneReport, prune_cache};

use std::{
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub(crate) fn root(cache_home: Option<&Path>, directory: &str) -> io::Result<PathBuf> {
    let root = resolve_cache_root(cache_home).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "failed to resolve XDG cache directory",
        )
    })?;
    Ok(root.join(directory))
}

/// resolves Veila's cache root without filesystem access, preferring an explicit cache home
pub fn resolve_cache_root(cache_home: Option<&Path>) -> Option<PathBuf> {
    let base = cache_home
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("veila"))
}

pub(crate) fn stable_hash(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(crate) fn file_cache_key(path: &Path) -> io::Result<String> {
    let metadata = fs::metadata(path)?;
    Ok(format!(
        "file:v2:{}:{}:{}:{}:{}:{}:{}",
        env!("CARGO_PKG_VERSION"),
        path.display(),
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{file_cache_key, resolve_cache_root, root, stable_hash};
    use std::{
        fs::{self, File, FileTimes},
        path::Path,
        time::{Duration, UNIX_EPOCH},
    };

    #[test]
    fn cache_hash_preserves_fnv1a_disk_keys() {
        assert_eq!(stable_hash(""), 0xcbf29ce484222325);
        assert_eq!(stable_hash("hello"), 0xa430d84680aabd0b);
    }

    #[test]
    fn explicit_cache_home_keeps_caches_separate() {
        for directory in ["backgrounds", "source-images", "avatars"] {
            assert_eq!(
                root(Some(Path::new("/tmp/cache")), directory).unwrap(),
                Path::new("/tmp/cache/veila").join(directory)
            );
        }
    }

    #[test]
    fn cache_root_preserves_explicit_native_path_bytes() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let cache_home = Path::new(OsStr::from_bytes(b"/tmp/veila-cache-\xff"));
        assert_eq!(
            resolve_cache_root(Some(cache_home)),
            Some(cache_home.join("veila"))
        );
    }

    #[test]
    fn resolving_cache_root_does_not_create_a_directory() {
        let cache_home =
            std::env::temp_dir().join(format!("veila-resolve-root-{}", std::process::id()));
        assert!(!cache_home.exists());
        assert_eq!(
            resolve_cache_root(Some(&cache_home)),
            Some(cache_home.join("veila"))
        );
        assert!(!cache_home.exists());
    }

    #[test]
    fn file_key_changes_for_a_same_second_edit() {
        let root = std::env::temp_dir().join(format!("veila-file-key-time-{}", std::process::id()));
        fs::create_dir_all(&root).expect("directory");
        let path = root.join("image.png");
        fs::write(&path, b"one").expect("image");
        let file = File::options().write(true).open(&path).expect("image file");
        let second = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        file.set_times(FileTimes::new().set_modified(second + Duration::from_nanos(1)))
            .expect("first timestamp");
        let first = file_cache_key(&path).expect("first key");

        fs::write(&path, b"two").expect("replacement content");
        file.set_times(FileTimes::new().set_modified(second + Duration::from_nanos(2)))
            .expect("second timestamp");

        assert_ne!(file_cache_key(&path).expect("second key"), first);
        drop(file);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn file_key_changes_for_same_size_same_time_replacement() {
        let root =
            std::env::temp_dir().join(format!("veila-file-key-inode-{}", std::process::id()));
        fs::create_dir_all(&root).expect("directory");
        let path = root.join("image.png");
        let replacement = root.join("replacement.png");
        fs::write(&path, b"one").expect("image");
        fs::write(&replacement, b"two").expect("replacement");
        let modified = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        for file_path in [&path, &replacement] {
            File::options()
                .write(true)
                .open(file_path)
                .expect("image file")
                .set_times(FileTimes::new().set_modified(modified))
                .expect("timestamp");
        }
        let first = file_cache_key(&path).expect("first key");

        fs::rename(&replacement, &path).expect("replace image");

        assert_ne!(file_cache_key(&path).expect("second key"), first);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
