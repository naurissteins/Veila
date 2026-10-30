use std::path::Path;

use veila_common::NowPlayingSnapshot;

use crate::{
    adapters::{
        ownership,
        process::{self, CurtainExit},
    },
    domain::lock_state::LockState,
};
use veila_auth::policy::AuthPolicy;

use super::super::{runtime::reset_runtime, state::RuntimeSlots};

pub(crate) async fn handle_curtain_exit(
    status: CurtainExit,
    slots: RuntimeSlots<'_>,
    auth_policy: AuthPolicy,
) {
    let RuntimeSlots {
        state,
        active,
        auth_state,
    } = slots;

    match status {
        CurtainExit::Spawned(exit_status) => {
            tracing::warn!(?exit_status, state = %state, "curtain exited");
        }
        CurtainExit::Adopted => {
            tracing::warn!(state = %state, "adopted curtain exited");
        }
    }
    let owner_path = active
        .as_ref()
        .map(|active| active.curtain.owner_path().to_path_buf());
    reset_runtime(active, auth_policy, auth_state);

    if state.is_active() {
        tracing::error!("curtain exited while locked; preserving unresolved lock ownership");
    } else if let Some(path) = owner_path
        && let Err(error) = ownership::remove(&path)
    {
        tracing::warn!("failed to clear released curtain ownership: {error:#}");
    }
}

pub(crate) async fn handle_now_playing_update(
    state: &LockState,
    control_socket_path: Option<&Path>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
) {
    if !state.is_active() {
        return;
    }

    let Some(control_socket_path) = control_socket_path else {
        tracing::debug!("ignoring now playing update without active curtain control socket");
        return;
    };

    if let Err(error) =
        process::request_curtain_now_playing_update(control_socket_path, now_playing_snapshot).await
    {
        tracing::warn!("failed to forward live now playing update to curtain: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::runtime::Fixture;
    use veila_auth::policy::AuthState;

    #[tokio::test]
    async fn active_curtain_exit_clears_resources_without_authorizing_unlock() {
        for initial in [LockState::Locking, LockState::Locked, LockState::Unlocking] {
            let mut fixture = Fixture::new();
            let sender = fixture.active.as_ref().expect("active").auth_sender.clone();
            let mut state = initial;
            let mut auth_state = AuthState::default();
            handle_curtain_exit(
                CurtainExit::Adopted,
                RuntimeSlots {
                    state: &mut state,
                    active: &mut fixture.active,
                    auth_state: &mut auth_state,
                },
                AuthPolicy::default(),
            )
            .await;
            assert_eq!(state, initial);
            assert!(fixture.active.is_none());
            assert!(sender.is_closed());
            assert!(fixture.root.join("owner.json").exists());
        }
    }

    #[tokio::test]
    async fn released_curtain_exit_removes_ownership_and_resources() {
        let mut fixture = Fixture::new();
        let mut state = LockState::Unlocked;
        let mut auth_state = AuthState::default();
        handle_curtain_exit(
            CurtainExit::Adopted,
            RuntimeSlots {
                state: &mut state,
                active: &mut fixture.active,
                auth_state: &mut auth_state,
            },
            AuthPolicy::default(),
        )
        .await;
        assert_eq!(state, LockState::Unlocked);
        assert!(fixture.active.is_none());
        assert!(!fixture.root.join("owner.json").exists());
        assert!(!fixture.root.join("auth.sock").exists());
    }
}
