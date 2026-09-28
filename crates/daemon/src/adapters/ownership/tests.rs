use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{OwnerRecord, load, parse_process_stat, process_matches, publish, remove};

fn fixture() -> (PathBuf, PathBuf) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("veila-owner-{}-{stamp}", std::process::id()));
    fs::create_dir(&directory).expect("create fixture directory");
    let path = directory.join("curtain-owner-_32.json");
    (directory, path)
}

#[test]
fn process_stat_parser_handles_parentheses_in_name() {
    let mut fields = vec!["S"; 20];
    fields[19] = "12345";
    let stat = format!("42 (curtain ) name) {}", fields.join(" "));
    assert_eq!(parse_process_stat(&stat).expect("start time"), ('S', 12345));
}

#[test]
fn published_record_preserves_process_and_socket_identity() {
    let (directory, path) = fixture();
    let mut owner = OwnerRecord::pending(
        "/org/freedesktop/login1/session/_32",
        directory.join("auth.sock"),
        directory.join("control.sock"),
    );
    publish(&path, &owner).expect("publish pending owner");
    assert!(
        load(&path, &owner.session)
            .expect("load pending owner")
            .expect("owner")
            .pid
            .is_none()
    );
    owner
        .set_process(std::process::id())
        .expect("set current process");
    publish(&path, &owner).expect("publish live owner");
    let loaded = load(&path, &owner.session)
        .expect("load live owner")
        .expect("owner");
    assert_eq!(loaded.pid, Some(std::process::id()));
    assert!(loaded.gate_was_closed());
    owner.mark_gate_opening();
    publish(&path, &owner).expect("publish gate transition");
    assert!(
        !load(&path, &owner.session)
            .expect("load gate transition")
            .expect("owner")
            .gate_was_closed()
    );
    assert!(
        process_matches(
            loaded.pid.expect("pid"),
            loaded.start_ticks.expect("start time")
        )
        .expect("process check")
    );
    assert!(load(&path, "/different/session").is_err());
    remove(&path).expect("remove owner");
    fs::remove_dir(&directory).expect("remove fixture directory");
}
