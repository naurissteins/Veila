use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn root() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "veila-process-status-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).expect("directory");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private");
    path
}
#[test]
fn status_rejects_reused_process_identity_and_ignores_unheld_stale_record() {
    let root = root();
    let display = PathBuf::from("display");
    let mut record =
        record::LockRecord::at(root.clone(), display.clone(), Some("config.toml".into()))
            .expect("record")
            .expect("lock");
    record.value.state = State::Locked;
    record.value.curtain_pid = Some(std::process::id());
    record.value.curtain_start_ticks =
        Some(record::start_ticks(std::process::id()).expect("ticks"));
    record.save().expect("save");
    let status = status_at(root.clone(), display.clone()).expect("status");
    assert_eq!(status.state, "locked");
    assert_eq!(
        status.supervisor.expect("supervisor").config,
        Some("config.toml".into())
    );
    assert_eq!(status.curtain, Some(std::process::id()));
    record.value.supervisor_start_ticks += 1;
    record.value.curtain_start_ticks = Some(0);
    record.save().expect("save");
    let status = status_at(root.clone(), display.clone()).expect("status");
    assert!(status.supervisor.is_none() && status.curtain.is_none());
    let inherited = record.file.try_clone().expect("inherit");
    drop(record);
    assert_eq!(
        status_at(root.clone(), display.clone())
            .expect("held")
            .state,
        "locked"
    );
    drop(inherited);
    assert_eq!(
        status_at(root.clone(), display).expect("stale").state,
        "unlocked"
    );
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn watcher_status_requires_held_record_and_matching_start_time() {
    let root = root();
    let display = PathBuf::from("display");
    let mut file = record::open_record(root.clone(), display.clone(), "idle").expect("record");
    file.try_lock().expect("lock");
    let mut identity = WatcherIdentity {
        pid: std::process::id(),
        start_ticks: record::start_ticks(std::process::id()).expect("ticks"),
        config: None,
    };
    file.write_all(&serde_json::to_vec(&identity).expect("serialize"))
        .expect("write");
    assert_eq!(
        status_at(root.clone(), display.clone())
            .expect("status")
            .idle
            .expect("watcher")
            .pid,
        std::process::id()
    );
    identity.start_ticks += 1;
    file.rewind().expect("rewind");
    let bytes = serde_json::to_vec(&identity).expect("serialize");
    file.write_all(&bytes).expect("write");
    file.set_len(bytes.len() as u64).expect("truncate");
    assert!(
        status_at(root.clone(), display.clone())
            .expect("reused PID")
            .idle
            .is_none()
    );
    drop(file);
    assert!(
        status_at(root.clone(), display)
            .expect("stale")
            .idle
            .is_none()
    );
    fs::remove_dir_all(root).expect("cleanup");
}
