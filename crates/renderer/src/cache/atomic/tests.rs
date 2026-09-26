use super::write_atomic;
use std::{
    fs,
    io::{self, Write},
    sync::{Arc, Barrier},
    thread,
};

#[test]
fn concurrent_writes_publish_only_complete_files() {
    let root = std::env::temp_dir().join(format!("veila-atomic-race-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("entry.argb");
    write_atomic(&path, |file| file.write_all(&vec![0; 65_536])).unwrap();
    let barrier = Arc::new(Barrier::new(5));
    thread::scope(|scope| {
        for value in 1..=4 {
            let path = &path;
            let barrier = Arc::clone(&barrier);
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..12 {
                    write_atomic(path, |file| {
                        file.write_all(&vec![value; 32_768])?;
                        thread::yield_now();
                        file.write_all(&vec![value; 32_768])
                    })
                    .unwrap();
                }
            });
        }
        barrier.wait();
        for _ in 0..200 {
            let bytes = fs::read(&path).unwrap();
            assert_eq!(bytes.len(), 65_536);
            assert!(bytes.iter().all(|byte| *byte == bytes[0]));
            thread::yield_now();
        }
    });
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_write_preserves_previous_file_and_removes_temporary_file() {
    let root = std::env::temp_dir().join(format!("veila-atomic-fail-{}", std::process::id()));
    let path = root.join("entry.rgba");
    write_atomic(&path, |file| file.write_all(b"previous")).unwrap();
    let result = write_atomic(&path, |file| {
        file.write_all(b"partial")?;
        Err(io::Error::other("injected write failure"))
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), b"previous");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixed_temporary_symlink_is_not_followed() {
    let root = std::env::temp_dir().join(format!("veila-atomic-symlink-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let target = root.join("unrelated");
    fs::write(&target, b"untouched").unwrap();
    std::os::unix::fs::symlink(&target, root.join(".entry.argb.tmp")).unwrap();
    let path = root.join("entry.argb");
    write_atomic(&path, |file| file.write_all(b"cached")).unwrap();
    assert_eq!(fs::read(target).unwrap(), b"untouched");
    assert_eq!(fs::read(path).unwrap(), b"cached");
    fs::remove_dir_all(root).unwrap();
}
