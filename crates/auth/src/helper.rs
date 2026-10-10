mod conversation;
pub(crate) mod service;

use std::process::{Command, Stdio};

use nix::{
    sys::{prctl, signal::Signal},
    unistd::getppid,
};

use crate::protocol::{PARENT_PID_ENV, ProtocolError};

pub const PAM_HELPER_SUBCOMMAND: &str = "__pam-helper";
pub const PAM_HELPER_PROCESS_NAME: &str = "veila-pam-helper";

#[derive(Debug, thiserror::Error)]
pub enum HelperError {
    #[error("PAM helper parent identity is missing or invalid")]
    ParentIdentity,
    #[error("failed to arm PAM helper parent-death signal")]
    ParentDeathSignal(#[source] nix::Error),
    #[error("PAM helper parent exited before startup")]
    ParentExited,
    #[error("PAM helper expected a valid start request")]
    InvalidStart,
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
}

/// Runs the `__pam-helper` mode: one PAM transaction driven over stdin/stdout.
pub fn run_helper() -> Result<(), HelperError> {
    let expected_parent = std::env::var(PARENT_PID_ENV)
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .ok_or(HelperError::ParentIdentity)?;
    prctl::set_pdeathsig(Signal::SIGKILL).map_err(HelperError::ParentDeathSignal)?;
    if getppid().as_raw() != expected_parent {
        return Err(HelperError::ParentExited);
    }
    conversation::run()
}

/// Builds the command that re-executes this binary as a PAM helper for one attempt.
pub fn helper_command() -> Command {
    let mut command = Command::new("/proc/self/exe");
    std::os::unix::process::CommandExt::arg0(&mut command, PAM_HELPER_PROCESS_NAME);
    command
        .arg(PAM_HELPER_SUBCOMMAND)
        .env(PARENT_PID_ENV, std::process::id().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

pub fn report_service_selection() {
    #[cfg(debug_assertions)]
    if let Ok(service) = std::env::var("VEILA_PAM_SERVICE") {
        tracing::warn!(service, "debug PAM service override is active");
        return;
    }

    match service::selected_service() {
        Some(selected) if selected.fallback => tracing::warn!(
            service = selected.name,
            "Veila PAM service is missing; using fallback"
        ),
        Some(_) => {}
        None => tracing::error!(
            "no Veila PAM service or supported fallback exists in /etc/pam.d; password authentication is unavailable"
        ),
    }
}

fn pam_service() -> Option<String> {
    #[cfg(debug_assertions)]
    if let Ok(service) = std::env::var("VEILA_PAM_SERVICE") {
        tracing::warn!(service, "using debug PAM service override");
        return Some(service);
    }
    let Some(selected) = service::selected_service() else {
        tracing::error!(
            "PAM service selection failed: no Veila PAM service or supported fallback exists in /etc/pam.d"
        );
        return None;
    };
    if selected.fallback {
        tracing::warn!(
            service = selected.name,
            "Veila PAM service missing; using fallback"
        );
    }
    Some(selected.name.to_owned())
}
