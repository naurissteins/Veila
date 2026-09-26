use crate::{RendererError, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const STALE_TEMP_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheKind {
    RenderedBackground,
    SourceImage,
    Avatar,
}

impl CacheKind {
    pub const fn directory(self) -> &'static str {
        match self {
            Self::RenderedBackground => "backgrounds",
            Self::SourceImage => "source-images",
            Self::Avatar => "avatars",
        }
    }
    const fn extension(self) -> &'static str {
        match self {
            Self::RenderedBackground => "argb",
            Self::SourceImage | Self::Avatar => "rgba",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachePrunePolicy {
    pub max_bytes: u64,
    pub max_age: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CachePruneReport {
    pub scanned_files: usize,
    pub removed_files: usize,
    pub removed_bytes: u64,
    pub retained_bytes: u64,
}

#[derive(Debug)]
struct PruneEntry {
    path: PathBuf,
    byte_len: u64,
    modified: SystemTime,
}

pub fn prune_cache(kind: CacheKind, policy: CachePrunePolicy) -> Result<CachePruneReport> {
    prune_cache_at(kind, policy, None, SystemTime::now())
}

fn prune_cache_at(
    kind: CacheKind,
    policy: CachePrunePolicy,
    cache_home: Option<&Path>,
    now: SystemTime,
) -> Result<CachePruneReport> {
    let root = super::root(cache_home, kind.directory())?;
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CachePruneReport::default());
        }
        Err(error) => return Err(RendererError::Io(error)),
    };

    let mut report = CachePruneReport::default();
    let mut retained = Vec::new();

    for entry in entries {
        let entry = entry.map_err(RendererError::Io)?;
        let path = entry.path();
        let temporary = is_temporary_cache_file(&path, kind);
        if !temporary
            && path.extension().and_then(|extension| extension.to_str()) != Some(kind.extension())
        {
            continue;
        }

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(RendererError::Io(error)),
        };
        if !metadata.is_file() {
            continue;
        }

        let byte_len = metadata.len();
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
        if temporary {
            if now
                .duration_since(modified)
                .is_ok_and(|age| age > STALE_TEMP_AGE)
            {
                report.scanned_files += 1;
                remove_pruned_file(&path, byte_len, &mut report)?;
            }
            continue;
        }
        report.scanned_files += 1;

        if now
            .duration_since(modified)
            .is_ok_and(|age| age > policy.max_age)
        {
            remove_pruned_file(&path, byte_len, &mut report)?;
        } else {
            retained.push(PruneEntry {
                path,
                byte_len,
                modified,
            });
        }
    }

    let mut retained_bytes = retained
        .iter()
        .fold(0u64, |total, entry| total.saturating_add(entry.byte_len));
    retained.sort_by_key(|entry| entry.modified);

    for entry in retained {
        if retained_bytes <= policy.max_bytes {
            break;
        }

        remove_pruned_file(&entry.path, entry.byte_len, &mut report)?;
        retained_bytes = retained_bytes.saturating_sub(entry.byte_len);
    }

    report.retained_bytes = retained_bytes;
    Ok(report)
}

fn is_temporary_cache_file(path: &Path, kind: CacheKind) -> bool {
    let Some(stem) = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix('.'))
        .and_then(|name| name.strip_suffix(".tmp"))
    else {
        return false;
    };
    let mut parts = stem.split('.');
    let Some(key) = parts.next() else {
        return false;
    };
    if key.len() != 16
        || !key.bytes().all(|byte| byte.is_ascii_hexdigit())
        || parts.next() != Some(kind.extension())
    {
        return false;
    }
    match (parts.next(), parts.next(), parts.next()) {
        (None, None, None) => true,
        (Some(pid), Some(counter), None) => {
            pid.parse::<u32>().is_ok() && counter.parse::<u64>().is_ok()
        }
        _ => false,
    }
}

fn remove_pruned_file(path: &Path, byte_len: u64, report: &mut CachePruneReport) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => {
            report.removed_files += 1;
            report.removed_bytes = report.removed_bytes.saturating_add(byte_len);
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(RendererError::Io(error)),
    }
}

#[cfg(test)]
mod tests;
