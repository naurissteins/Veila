mod activation;
mod active;
mod auth;
mod state;
mod unlock;

pub(super) use activation::activate_lock;
pub(super) use active::{ActiveLock, control_socket_path};
pub(super) use auth::{AuthResult, ClientMessageContext, handle_client_message};
pub(super) use state::{
    accept_auth_connection, accept_control_connection, receive_auth_result, reset_runtime,
    update_locked_hint, wait_for_curtain_exit,
};
pub(super) use unlock::deactivate_lock;

#[cfg(test)]
pub(super) use active::tests::Fixture;
