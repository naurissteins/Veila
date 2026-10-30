use crate::{adapters::logind, domain::lock_state::LockState};
use veila_auth::policy::AuthPolicy;

use super::super::{runtime::deactivate_lock, state::RuntimeSlots};

#[derive(Default)]
pub(crate) struct ShutdownGate {
    requested: bool,
}

impl ShutdownGate {
    pub(crate) fn is_requested(&self) -> bool {
        self.requested
    }

    pub(crate) fn request(&mut self, state: LockState, curtain_running: bool) -> bool {
        self.requested = true;
        self.ready(state, curtain_running)
    }

    pub(crate) fn ready(&self, state: LockState, curtain_running: bool) -> bool {
        self.requested && !state.is_active() && !curtain_running
    }
}

pub(crate) async fn shutdown_runtime(
    session_proxy: &logind::SessionProxy<'_>,
    slots: RuntimeSlots<'_>,
    auth_policy: AuthPolicy,
) {
    let RuntimeSlots {
        state,
        active,
        auth_state,
    } = slots;

    if let Err(error) =
        deactivate_lock(session_proxy, state, active, auth_policy, auth_state, None).await
    {
        tracing::warn!("failed to stop curtain during shutdown: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::{LockState, ShutdownGate};

    #[test]
    fn signal_shutdown_waits_for_unlock() {
        let mut gate = ShutdownGate::default();
        assert!(!gate.request(LockState::Locking, true));
        assert!(!gate.ready(LockState::Locked, true));
        assert!(!gate.ready(LockState::Locked, false));
        assert!(!gate.ready(LockState::Unlocking, true));
        assert!(!gate.ready(LockState::Unlocked, true));
        assert!(gate.ready(LockState::Unlocked, false));
    }

    #[test]
    fn signal_shutdown_exits_when_already_unlocked() {
        let mut gate = ShutdownGate::default();
        assert!(gate.request(LockState::Unlocked, false));
    }
}
