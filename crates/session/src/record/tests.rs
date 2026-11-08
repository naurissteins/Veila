use super::*;
use std::{
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "veila-record-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).expect("root");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("mode");
    root
}
#[test]
fn inherited_file_keeps_record_locked_after_supervisor_drop() {
    let root = root();
    let display = PathBuf::from("wayland-test");
    let mut record = LockRecord::at(root.clone(), display.clone(), None)
        .expect("record")
        .expect("lock");
    let inherited = record.file.try_clone().expect("inherit");
    record.value.state = State::Locked;
    record.save().expect("save");
    drop(record);
    assert!(
        LockRecord::at(root.clone(), display.clone(), None)
            .expect("check")
            .is_none()
    );
    drop(inherited);
    let next = LockRecord::at(root.clone(), display, None)
        .expect("next")
        .expect("lock");
    assert!(next.recovered);
    drop(next);
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn unlocked_record_reuses_inode_and_displays_are_independent() {
    let root = root();
    let a = PathBuf::from("a");
    let mut record = LockRecord::at(root.clone(), a.clone(), None)
        .expect("record")
        .expect("lock");
    let inode = record.file.metadata().expect("meta").ino();
    let other = LockRecord::at(root.clone(), "b".into(), None)
        .expect("other")
        .expect("lock");
    drop(other);
    record.value.state = State::Unlocked;
    record.save().expect("save");
    drop(record);
    let next = LockRecord::at(root.clone(), a, None)
        .expect("next")
        .expect("lock");
    assert!(!next.recovered);
    assert_eq!(inode, next.file.metadata().expect("meta").ino());
    drop(next);
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn rejects_insecure_or_symlinked_directories() {
    let root = root();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).expect("mode");
    assert!(LockRecord::at(root.clone(), "a".into(), None).is_err());
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("mode");
    std::os::unix::fs::symlink("/tmp", root.join("veila")).expect("symlink");
    assert!(LockRecord::at(root.clone(), "a".into(), None).is_err());
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn torn_non_utf8_record_retains_recovery_backoff_instead_of_preventing_lock() {
    let root = root();
    let display = PathBuf::from("wayland-test");
    let mut record = LockRecord::at(root.clone(), display.clone(), None)
        .expect("record")
        .expect("lock");
    record.file.seek(SeekFrom::Start(0)).expect("seek");
    record
        .file
        .write_all(b"{\"config\":\"\xff")
        .expect("torn record");
    record.file.set_len(12).expect("truncate");
    drop(record);
    let recovered = LockRecord::at(root.clone(), display, None)
        .expect("read malformed bytes")
        .expect("lock");
    assert!(recovered.recovered);
    drop(recovered);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn probe_distinguishes_confirmation_and_never_mutates_stale_records() {
    let root = root();
    let display = PathBuf::from("probe");
    let mut record = LockRecord::at(root.clone(), display.clone(), None)
        .expect("record")
        .expect("owner");
    assert_eq!(
        LockRecord::active_at(root.clone(), display.clone()).expect("probe"),
        Some(State::Starting)
    );
    record.value.state = State::Locked;
    record.save().expect("save");
    let inherited = record.file.try_clone().expect("inherit");
    drop(record);
    assert_eq!(
        LockRecord::active_at(root.clone(), display.clone()).expect("probe"),
        Some(State::Locked)
    );
    let before = fs::read_dir(root.join("veila"))
        .expect("directory")
        .next()
        .expect("file")
        .expect("entry")
        .path();
    let bytes = fs::read(&before).expect("bytes");
    drop(inherited);
    assert_eq!(
        LockRecord::active_at(root.clone(), display).expect("stale probe"),
        None
    );
    assert_eq!(fs::read(before).expect("bytes"), bytes);
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn torn_held_record_does_not_confirm_a_lock() {
    let root = root();
    let display = PathBuf::from("probe");
    let mut record = LockRecord::at(root.clone(), display.clone(), None)
        .expect("record")
        .expect("owner");
    record.file.set_len(0).expect("truncate");
    record.file.write_all(b"{incomplete").expect("write");
    assert_eq!(
        LockRecord::active_at(root.clone(), display).expect("probe"),
        Some(State::Starting)
    );
    drop(record);
    fs::remove_dir_all(root).expect("cleanup");
}
