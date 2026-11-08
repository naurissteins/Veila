use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn root() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "veila-duplicate-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).expect("directory");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("mode");
    path
}
fn owner(root: &std::path::Path) -> LockRecord {
    let mut record = LockRecord::at(root.into(), "display".into(), None)
        .expect("record")
        .expect("owner");
    record.value.curtain_pid = Some(std::process::id());
    record.value.curtain_start_ticks =
        Some(record::start_ticks(std::process::id()).expect("ticks"));
    record.save().expect("save");
    record
}
fn request(root: &std::path::Path, wait: bool, timeout: Duration) -> Result<Acquisition> {
    acquire_at(root.into(), "display".into(), None, wait, timeout)
}
#[test]
fn duplicate_waits_for_confirmation_and_rich_readiness() {
    let root = root();
    let mut owner = owner(&root);
    let began = Instant::now();
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(40));
            owner.value.state = State::Locked;
            owner.save().expect("confirm");
            std::thread::sleep(Duration::from_millis(40));
            owner.value.ready = Some(true);
            owner.save().expect("ready");
            owner
        });
        assert!(matches!(
            request(&root, true, Duration::from_secs(2)).expect("ready"),
            Acquisition::Duplicate { locked: true }
        ));
        assert!(began.elapsed() >= Duration::from_millis(70));
        drop(worker.join().expect("owner"));
    });
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn closing_owner_is_not_mistaken_for_an_active_lock() {
    let root = root();
    let mut owner = owner(&root);
    owner.value.state = State::Unlocked;
    owner.save().expect("unlocked");
    std::thread::scope(|scope| {
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(40));
            drop(owner);
        });
        let Acquisition::Owner(next) = request(&root, true, Duration::from_secs(2)).expect("next")
        else {
            panic!("new request was discarded");
        };
        assert!(!next.recovered);
        assert_eq!(next.value.state, State::Starting);
    });
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn release_during_startup_fails_without_relocking() {
    let root = root();
    let mut owner = owner(&root);
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(40));
            owner.value.state = State::Unlocked;
            owner.save().expect("release");
            owner
        });
        assert!(request(&root, true, Duration::from_secs(2)).is_err());
        let owner = worker.join().expect("owner");
        assert_eq!(owner.value.state, State::Unlocked);
    });
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn surviving_curtain_can_confirm_ready_without_a_supervisor() {
    let root = root();
    let mut owner = owner(&root);
    owner.value.state = State::Locked;
    owner.value.ready = Some(true);
    owner.value.supervisor_pid = i32::MAX as u32;
    owner.value.supervisor_start_ticks = 0;
    owner.save().expect("save");
    assert!(matches!(
        request(&root, true, Duration::from_secs(1)).expect("ready"),
        Acquisition::Duplicate { locked: true }
    ));
    drop(owner);
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn stale_process_identity_and_old_readiness_schema_cannot_confirm_ready() {
    let root = root();
    let mut owner = owner(&root);
    owner.value.state = State::Locked;
    owner.value.ready = Some(true);
    owner.value.curtain_start_ticks = Some(0);
    owner.save().expect("save");
    assert!(request(&root, true, Duration::from_millis(30)).is_err());
    owner.value.ready = None;
    owner.save().expect("old record");
    assert!(request(&root, true, Duration::from_secs(1)).is_err());
    drop(owner);
    fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn nonwaiting_duplicate_does_not_claim_confirmation_during_startup() {
    let root = root();
    let owner = owner(&root);
    assert!(matches!(
        request(&root, false, Duration::from_secs(1)).expect("duplicate"),
        Acquisition::Duplicate { locked: false }
    ));
    drop(owner);
    fs::remove_dir_all(root).expect("cleanup");
}
