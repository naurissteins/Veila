use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

const PRIVATE_DIR_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;

pub fn write_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
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
    use super::{PRIVATE_DIR_MODE, PRIVATE_FILE_MODE, write_private_file};
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
    };

    #[test]
    fn private_write_tightens_existing_cache_directory_and_file() {
        let root = std::env::temp_dir().join(format!("veila-private-cache-{}", std::process::id()));
        let cache_root = root.join("veila");
        let directory = cache_root.join("weather");
        fs::create_dir_all(&directory).expect("cache directory");
        fs::set_permissions(&cache_root, fs::Permissions::from_mode(0o755)).expect("root mode");
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).expect("directory mode");
        let path = directory.join("entry.json");
        fs::write(&path, b"old").expect("old cache");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("old mode");

        write_private_file(&path, b"new").expect("private cache write");

        assert_eq!(
            fs::metadata(&cache_root).expect("root metadata").mode() & 0o777,
            PRIVATE_DIR_MODE
        );
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
}
