use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;
const BACKUP_COUNT: usize = 3;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub struct RotatingLog {
    file: File,
    path: PathBuf,
    regular: bool,
    warned: bool,
}

impl RotatingLog {
    pub fn new(file: File, path: &Path) -> io::Result<Self> {
        let metadata = file.metadata()?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            regular: metadata.is_file(),
            warned: false,
        })
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        let (temporary_path, mut temporary) = self.create_temporary()?;
        let result = (|| {
            let mut source = File::open(&self.path)?;
            let source_size = source.metadata()?.len();
            source.seek(SeekFrom::Start(source_size.saturating_sub(MAX_LOG_BYTES)))?;
            io::copy(&mut source.take(MAX_LOG_BYTES), &mut temporary)?;
            temporary.flush()?;
            drop(temporary);

            fs::remove_file(backup_path(&self.path, BACKUP_COUNT)).or_else(ignore_missing)?;
            for index in (1..BACKUP_COUNT).rev() {
                fs::rename(
                    backup_path(&self.path, index),
                    backup_path(&self.path, index + 1),
                )
                .or_else(ignore_missing)?;
            }
            fs::rename(&temporary_path, backup_path(&self.path, 1))?;
            // keep the active inode so `tail -f` survives rotation
            self.file.set_len(0)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }

    fn create_temporary(&self) -> io::Result<(PathBuf, File)> {
        for _ in 0..8 {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = suffixed_path(
                &self.path,
                format!(".rotate-{}-{id}.tmp", std::process::id()),
            );
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a temporary log backup",
        ))
    }

    fn rotate_if_full(&mut self, incoming: usize) -> io::Result<()> {
        let size = self.file.metadata()?.len();
        if size > 0 && size.saturating_add(incoming as u64) > MAX_LOG_BYTES {
            if let Err(error) = self.rotate() {
                if !self.warned {
                    self.warned = true;
                    tracing::warn!(path = %self.path.display(), %error, "could not rotate lock log");
                }
            } else {
                self.warned = false;
            }
        }
        Ok(())
    }
}

impl Write for RotatingLog {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.regular || buf.is_empty() {
            return self.file.write(buf);
        }
        // multiple Wayland sessions can point their log writers at the same file
        self.file.lock()?;
        let result = self
            .rotate_if_full(buf.len())
            .and_then(|()| self.file.write_all(buf));
        let unlocked = self.file.unlock();
        result.and(unlocked)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn backup_path(path: &Path, index: usize) -> PathBuf {
    suffixed_path(path, format!(".{index}"))
}

fn suffixed_path(path: &Path, suffix: String) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

fn ignore_missing(error: io::Error) -> io::Result<()> {
    if error.kind() == io::ErrorKind::NotFound {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, os::unix::fs::MetadataExt, sync::Barrier, thread};

    use super::{RotatingLog, backup_path};
    use crate::logging::open_private_log;

    #[test]
    fn rotates_private_backups_without_replacing_active_inode() {
        let root = std::env::temp_dir().join(format!("veila-log-rotation-{}", std::process::id()));
        fs::create_dir_all(&root).expect("log directory");
        let path = root.join("veila.log");
        let mut log =
            RotatingLog::new(open_private_log(&path).expect("log"), &path).expect("writer");
        let inode = fs::metadata(&path).expect("metadata").ino();

        for marker in *b"abcde" {
            let entry = vec![marker; 5 * 1024 * 1024];
            log.write_all(&entry).expect("write entry");
        }

        assert_eq!(fs::metadata(&path).expect("metadata").ino(), inode);
        assert_eq!(
            fs::read(&path).expect("active"),
            vec![b'e'; 5 * 1024 * 1024]
        );
        for (index, marker) in [(1, b'd'), (2, b'c'), (3, b'b')] {
            let backup = backup_path(&path, index);
            assert_eq!(
                fs::read(&backup).expect("backup"),
                vec![marker; 5 * 1024 * 1024]
            );
            assert_eq!(fs::metadata(&backup).expect("mode").mode() & 0o777, 0o600);
        }
        assert_eq!(fs::metadata(&path).expect("mode").mode() & 0o777, 0o600);
        drop(log);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn failed_rotation_keeps_appending_to_active_log() {
        let root = std::env::temp_dir().join(format!("veila-log-failure-{}", std::process::id()));
        fs::create_dir_all(&root).expect("log directory");
        let path = root.join("veila.log");
        let mut log =
            RotatingLog::new(open_private_log(&path).expect("log"), &path).expect("writer");
        log.write_all(&vec![b'a'; 8 * 1024 * 1024])
            .expect("first write");
        fs::create_dir(backup_path(&path, 3)).expect("block rotation");

        log.write_all(b"after failure").expect("fallback append");
        assert_eq!(
            fs::metadata(&path).expect("metadata").len(),
            8 * 1024 * 1024 + 13
        );
        assert!(
            fs::read(&path)
                .expect("contents")
                .ends_with(b"after failure")
        );
        drop(log);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn large_existing_log_only_keeps_the_latest_backup_bytes() {
        let root = std::env::temp_dir().join(format!("veila-old-log-{}", std::process::id()));
        fs::create_dir_all(&root).expect("log directory");
        let path = root.join("veila.log");
        fs::write(&path, vec![b'a'; 9 * 1024 * 1024]).expect("existing log");
        let mut log =
            RotatingLog::new(open_private_log(&path).expect("log"), &path).expect("writer");

        log.write_all(b"new entry").expect("rotation and write");

        assert_eq!(
            fs::metadata(backup_path(&path, 1)).expect("backup").len(),
            8 * 1024 * 1024
        );
        assert_eq!(fs::read(&path).expect("active"), b"new entry");
        drop(log);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn concurrent_log_writers_serialize_rotation_and_keep_appending() {
        let root = std::env::temp_dir().join(format!("veila-shared-log-{}", std::process::id()));
        fs::create_dir_all(&root).expect("log directory");
        let path = root.join("veila.log");
        let first = open_private_log(&path).expect("first log");
        let second = open_private_log(&path).expect("second log");
        let inode = first.metadata().expect("metadata").ino();
        let start = Barrier::new(2);
        let chunk = 2 * 1024 * 1024;

        thread::scope(|scope| {
            for (file, marker) in [(first, b'a'), (second, b'b')] {
                let (start, path) = (&start, &path);
                scope.spawn(move || {
                    let mut writer = RotatingLog::new(file, path).expect("writer");
                    let data = vec![marker; chunk];
                    start.wait();
                    for _ in 0..3 {
                        writer.write_all(&data).expect("append");
                    }
                });
            }
        });

        let active = fs::read(&path).expect("active");
        let backup = fs::read(backup_path(&path, 1)).expect("backup");
        assert_eq!(active.len(), 2 * chunk);
        assert_eq!(backup.len(), 4 * chunk);
        assert!(!backup_path(&path, 2).exists());
        assert_eq!(fs::metadata(&path).expect("metadata").ino(), inode);
        for marker in *b"ab" {
            assert_eq!(
                backup
                    .iter()
                    .chain(&active)
                    .filter(|&&byte| byte == marker)
                    .count(),
                3 * chunk
            );
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}
