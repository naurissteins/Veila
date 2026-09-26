mod atomic;
pub(crate) mod image;
mod prune;
pub use prune::{CacheKind, CachePrunePolicy, CachePruneReport, prune_cache};

use std::{
    io,
    path::{Path, PathBuf},
};

pub(crate) fn root(cache_home: Option<&Path>, directory: &str) -> io::Result<PathBuf> {
    let base = cache_home
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "failed to resolve XDG cache directory",
            )
        })?;
    Ok(base.join("veila").join(directory))
}

pub(crate) fn stable_hash(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{root, stable_hash};
    use std::path::Path;

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
}
