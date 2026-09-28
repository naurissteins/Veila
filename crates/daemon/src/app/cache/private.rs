use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const PRIVATE_DIR_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;

pub(crate) fn harden_existing_cache_root() -> io::Result<()> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cache home is unavailable"))?;
    let root = base.join("veila");
    harden_existing_cache_root_at(&root)
}

fn harden_existing_cache_root_at(root: &Path) -> io::Result<()> {
    match fs::metadata(root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(io::Error::other("cache root is not a directory")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    secure_dir(root)?;
    for name in ["backgrounds", "source-images", "avatars", "weather"] {
        let child = root.join(name);
        if child.exists() {
            secure_dir(&child)?;
        }
    }
    Ok(())
}

pub(crate) fn write_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("cache path has no parent"))?;
    let root = parent
        .parent()
        .ok_or_else(|| io::Error::other("cache directory has no parent"))?;
    secure_dir(root)?;
    secure_dir(parent)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(PRIVATE_FILE_MODE)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))?;
    file.set_len(0)?;
    file.write_all(contents)
}

fn secure_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_DIR_MODE))
}

#[cfg(test)]
mod tests {
    use super::{
        PRIVATE_DIR_MODE, PRIVATE_FILE_MODE, harden_existing_cache_root_at, write_private_file,
    };
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
    };

    #[test]
    fn private_write_tightens_existing_cache_directory_and_file() {
        let root = std::env::temp_dir().join(format!("veila-private-cache-{}", std::process::id()));
        let directory = root.join("veila/weather");
        fs::create_dir_all(&directory).expect("cache directory");
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).expect("directory mode");
        let path = directory.join("entry.json");
        fs::write(&path, b"old").expect("old cache");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("old mode");

        write_private_file(&path, b"new").expect("private cache write");

        assert_eq!(
            fs::metadata(&directory).expect("directory metadata").mode() & 0o777,
            PRIVATE_DIR_MODE
        );
        assert_eq!(
            fs::metadata(&path).expect("file metadata").mode() & 0o777,
            PRIVATE_FILE_MODE
        );
        assert_eq!(fs::read(&path).expect("cache contents"), b"new");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn existing_cache_tree_becomes_private_on_daemon_start() {
        let root = std::env::temp_dir().join(format!("veila-private-root-{}", std::process::id()));
        let child = root.join("source-images");
        fs::create_dir_all(&child).expect("cache directory");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).expect("root mode");
        fs::set_permissions(&child, fs::Permissions::from_mode(0o755)).expect("child mode");

        harden_existing_cache_root_at(&root).expect("private cache tree");

        assert_eq!(
            fs::metadata(&root).expect("metadata").mode() & 0o777,
            PRIVATE_DIR_MODE
        );
        assert_eq!(
            fs::metadata(&child).expect("metadata").mode() & 0o777,
            PRIVATE_DIR_MODE
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
