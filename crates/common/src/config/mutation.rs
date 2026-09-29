use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use toml_edit::{Document, DocumentMut, value};

use crate::error::{Result, VeilaError};

use super::{resolve_theme_path, user_config_path, validate_theme_name};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub fn set_theme_in_config(explicit_path: Option<&Path>, theme: &str) -> Result<PathBuf> {
    validate_theme_name(theme)?;
    let path = config_path(explicit_path)?;
    resolve_theme_path(theme, path.parent())?;

    let mut document = if path.exists() {
        fs::read_to_string(&path)?.parse::<DocumentMut>()?
    } else {
        DocumentMut::new()
    };
    let old_decor = document
        .get("theme")
        .and_then(toml_edit::Item::as_value)
        .map(|old| old.decor().clone());
    let mut new_theme = value(theme);
    if let (Some(decor), Some(new_value)) = (old_decor, new_theme.as_value_mut()) {
        *new_value.decor_mut() = decor;
    }
    document["theme"] = new_theme;
    write_config_file(&path, &document.to_string(), true)?;
    Ok(path)
}

pub fn init_config(explicit_path: Option<&Path>, theme: &str, force: bool) -> Result<PathBuf> {
    validate_theme_name(theme)?;
    let path = config_path(explicit_path)?;
    resolve_theme_path(theme, path.parent())?;

    if path.exists() && !force {
        return Err(VeilaError::ConfigIo(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "config already exists; pass --force to replace it",
        )));
    }

    let mut document = DocumentMut::new();
    document["theme"] = value(theme);
    write_config_file(&path, &document.to_string(), force)?;
    Ok(path)
}

pub fn unset_theme_in_config(explicit_path: Option<&Path>) -> Result<(PathBuf, bool)> {
    let path = config_path(explicit_path)?;
    if !path.exists() {
        return Ok((path, false));
    }

    let raw = fs::read_to_string(&path)?;
    let document = Document::parse(raw.as_str())?;
    let Some(theme) = document.get("theme") else {
        return Ok((path, false));
    };
    if theme.as_value().is_none() {
        return Err(VeilaError::ConfigIo(io::Error::new(
            io::ErrorKind::InvalidData,
            "top-level theme must be a TOML value",
        )));
    }
    let start = document
        .as_table()
        .key("theme")
        .and_then(|key| key.span())
        .ok_or_else(|| io::Error::other("theme key has no source location"))?
        .start;
    let end = theme
        .span()
        .ok_or_else(|| io::Error::other("theme value has no source location"))?
        .end;
    let line_start = raw[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = raw[end..]
        .find('\n')
        .map_or(raw.len(), |index| end + index + 1);
    // Remove the assignment line while leaving nearby comments and formatting intact.
    let updated = format!("{}{}", &raw[..line_start], &raw[line_end..]);
    write_config_file(&path, &updated, true)?;
    Ok((path, true))
}

fn config_path(explicit_path: Option<&Path>) -> Result<PathBuf> {
    explicit_path.map(Path::to_path_buf).map_or_else(
        || {
            user_config_path().ok_or_else(|| {
                VeilaError::ConfigIo(io::Error::new(
                    io::ErrorKind::NotFound,
                    "failed to resolve default config path",
                ))
            })
        },
        Ok,
    )
}

fn write_config_file(path: &Path, contents: &str, replace: bool) -> Result<()> {
    write_config_file_inner(path, contents, replace).map_err(|error| map_write_error(path, error))
}

fn write_config_file_inner(path: &Path, contents: &str, replace: bool) -> io::Result<()> {
    let target = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path)?,
        Ok(_) => path.to_path_buf(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => path.to_path_buf(),
        Err(error) => return Err(error),
    };
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let mode = match fs::metadata(&target) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "config path is not a regular file",
                ));
            }
            if !replace {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "config already exists; pass --force to replace it",
                ));
            }
            // Check the original file's write access before replacing its inode.
            OpenOptions::new().write(true).open(&target)?;
            metadata.permissions().mode() & 0o777
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0o600,
        Err(error) => return Err(error),
    };

    let (temporary, mut file) = create_temporary(&target, parent)?;
    file.write_all(contents.as_bytes())?;
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    file.sync_all()?;
    drop(file);

    if replace {
        fs::rename(&temporary.0, &target)?;
    } else {
        // Linking publishes a new config only if no other writer created it.
        fs::hard_link(&temporary.0, &target)?;
    }
    Ok(())
}

fn create_temporary(path: &Path, parent: &Path) -> io::Result<(TemporaryFile, File)> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("config path has no filename"))?;
    for _ in 0..128 {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(name);
        temporary_name.push(format!(".{}.{id}.tmp", std::process::id()));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((TemporaryFile(temporary_path), file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "config temporary filename collisions",
    ))
}

fn map_write_error(path: &Path, error: io::Error) -> VeilaError {
    match error.kind() {
        io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem => {
            VeilaError::ConfigIo(io::Error::new(
                error.kind(),
                format!(
                    "cannot write {}: the config file is read-only, so it is likely managed declaratively; change it at its source instead",
                    path.display()
                ),
            ))
        }
        _ => VeilaError::ConfigIo(error),
    }
}

struct TemporaryFile(PathBuf);

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
