mod auth;
mod control;
mod lifecycle;
mod shutdown;

pub(super) use auth::{handle_auth_message, handle_auth_result};
pub(super) use control::handle_control_message;
pub(super) use lifecycle::{
    handle_curtain_exit, handle_lock_signal, handle_now_playing_update, handle_unlock_signal,
};
pub(super) use shutdown::shutdown_runtime;
