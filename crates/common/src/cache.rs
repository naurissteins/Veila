use std::path::{Path, PathBuf};
pub fn resolve_root(cache_home: Option<&Path>) -> Option<PathBuf> {
    let base = cache_home
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("veila"))
}
