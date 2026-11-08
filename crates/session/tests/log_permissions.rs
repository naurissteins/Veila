use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
};

use veila_session::logging::open_private_log;

#[test]
fn opening_an_existing_log_removes_public_read_access() {
    let root = std::env::temp_dir().join(format!("veila-private-log-{}", std::process::id()));
    fs::create_dir_all(&root).expect("log directory");
    let path = root.join("veila.log");
    fs::write(&path, b"old log\n").expect("old log");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("old mode");

    let file = open_private_log(&path).expect("private log");

    assert_eq!(file.metadata().expect("log metadata").mode() & 0o777, 0o600);
    assert_eq!(fs::read(&path).expect("log contents"), b"old log\n");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn new_log_file_is_owner_only() {
    let root = std::env::temp_dir().join(format!("veila-new-log-{}", std::process::id()));
    fs::create_dir_all(&root).expect("log directory");
    let path = root.join("veila.log");

    let file = open_private_log(&path).expect("private log");

    assert_eq!(file.metadata().expect("log metadata").mode() & 0o777, 0o600);
    fs::remove_dir_all(root).expect("cleanup");
}
