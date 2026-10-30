use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use tokio::{net::UnixListener, sync::mpsc::unbounded_channel};

use super::ActiveLock;
use crate::adapters::{ownership, process::CurtainHandle};
use crate::app::runtime::state::reset_runtime;
use veila_auth::policy::{AuthPolicy, AuthState};

pub(crate) struct Fixture {
    pub(crate) root: PathBuf,
    pub(crate) active: Option<ActiveLock>,
}

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

impl Fixture {
    pub(crate) fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "veila-active-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("fixture directory");
        let auth_socket_path = root.join("auth.sock");
        let control_socket_path = root.join("control.sock");
        let auth_listener = UnixListener::bind(&auth_socket_path).expect("auth fixture");
        let control_listener = UnixListener::bind(&control_socket_path).expect("control fixture");
        drop(control_listener);
        let owner_path = root.join("owner.json");
        fs::write(&owner_path, b"retained ownership").expect("ownership marker");
        let (auth_sender, auth_results) = unbounded_channel();
        let pid = std::process::id();
        let active = ActiveLock {
            curtain: CurtainHandle::Adopted {
                pid,
                start_ticks: ownership::process_start_ticks(pid).expect("process identity"),
                owner_path,
                next_check: tokio::time::Instant::now(),
            },
            auth_listener,
            auth_socket_path,
            control_socket_path,
            auth_sender,
            auth_results,
        };
        Self {
            root,
            active: Some(active),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.active.take();
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn reset_releases_all_resources_but_keeps_unresolved_ownership() {
    let mut fixture = Fixture::new();
    let sender = fixture.active.as_ref().expect("active").auth_sender.clone();
    let policy = AuthPolicy::default();
    let mut auth_state = AuthState::after_recovery(policy, Instant::now());
    reset_runtime(&mut fixture.active, policy, &mut auth_state);
    assert!(fixture.active.is_none());
    assert!(sender.is_closed());
    assert!(!fixture.root.join("auth.sock").exists());
    assert!(!fixture.root.join("control.sock").exists());
    assert!(fixture.root.join("owner.json").exists());
    assert!(matches!(
        auth_state.admit(Instant::now()),
        veila_auth::policy::AuthAdmission::Allowed
    ));
    reset_runtime(&mut fixture.active, policy, &mut auth_state);
    assert!(fixture.root.join("owner.json").exists());
}
