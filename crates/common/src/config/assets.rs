use std::path::{Path, PathBuf};

pub const ASSET_DIR_ENV: &str = "VEILA_ASSET_DIR";

pub(super) fn bundled_asset_dir() -> PathBuf {
    let local_assets = source_asset_dir();
    let system_assets = system_asset_dirs();

    for directory in asset_dir_candidates(env_asset_dir(), &local_assets, &system_assets) {
        if directory.exists() {
            return directory;
        }
    }

    local_assets
}

pub(super) fn bundled_background_dir() -> PathBuf {
    bundled_asset_dir().join("bg")
}

pub(super) fn bundled_theme_dir() -> PathBuf {
    bundled_asset_dir().join("themes")
}

fn asset_dir_candidates(
    env_assets: Option<PathBuf>,
    local_assets: &Path,
    system_assets: &[PathBuf],
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(env_assets) = env_assets {
        candidates.push(env_assets);
    }
    candidates.push(local_assets.to_path_buf());
    candidates.extend(system_assets.iter().cloned());
    candidates
}

fn env_asset_dir() -> Option<PathBuf> {
    let value = std::env::var_os(ASSET_DIR_ENV)?;
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

fn source_asset_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn system_asset_dirs() -> [PathBuf; 2] {
    [
        PathBuf::from("/usr/local/share/veila"),
        PathBuf::from("/usr/share/veila"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puts_env_asset_dir_before_source_and_system_paths() {
        let env_assets = Some(PathBuf::from("/nix/store/example-veila/share/veila"));
        let local_assets = PathBuf::from("/build/source/assets");
        let system_assets = [PathBuf::from("/usr/share/veila")];

        let candidates = asset_dir_candidates(env_assets, &local_assets, &system_assets);

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/nix/store/example-veila/share/veila"),
                PathBuf::from("/build/source/assets"),
                PathBuf::from("/usr/share/veila"),
            ]
        );
    }

    #[test]
    fn omits_missing_env_asset_dir_candidate() {
        let local_assets = PathBuf::from("/build/source/assets");
        let system_assets = [PathBuf::from("/usr/share/veila")];

        let candidates = asset_dir_candidates(None, &local_assets, &system_assets);

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/build/source/assets"),
                PathBuf::from("/usr/share/veila"),
            ]
        );
    }
}
