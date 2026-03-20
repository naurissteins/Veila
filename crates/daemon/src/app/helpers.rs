use anyhow::{Context, Result, anyhow};
use nix::unistd::{Uid, User};

use crate::{
    adapters::logind,
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};

use super::runtime::{ActiveRuntime, activate_lock};

pub(super) async fn activate_and_install(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) -> Result<()> {
    let activation = activate_lock(session_proxy, state, config_path).await?;
    runtime.install_activation(activation);
    *auth_state = AuthState::new(auth_policy);
    Ok(())
}

pub(super) fn current_username() -> Result<String> {
    let uid = Uid::current();
    let Some(user) = User::from_uid(uid).context("failed to resolve current username")? else {
        return Err(anyhow!("current uid {uid} does not resolve to a user"));
    };

    Ok(user.name)
}
