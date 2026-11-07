use anyhow::{Result, bail};

use crate::{CurtainOptions, help};

pub(super) fn validate_invocation_mode(options: &CurtainOptions) -> Result<()> {
    if options.initial_backoff_ms > 0 && options.uses_daemon_lock_flow() {
        bail!("initial backoff requires curtain-owned authentication");
    }
    // A partial ownership handshake must never reach Wayland startup.
    if options.owner_gate != options.owner_record.is_some() {
        bail!("daemon-managed curtain ownership requires both the startup gate and record");
    }
    if options.owner_gate
        && (options.notify_socket.is_none()
            || options.daemon_socket.is_none()
            || options.control_socket.is_none())
    {
        bail!("daemon-managed curtain ownership requires all daemon sockets");
    }
    if options.preview_png.is_some() || options.lock || options.uses_daemon_lock_flow() {
        return Ok(());
    }

    help::print_curtain_help();
    bail!(
        "refusing to start a real lock session from a plain `veila __curtain` launch; use `veila lock`, or pass `--lock` if you really want a direct curtain test"
    );
}

pub(super) fn validate_preview_mode(options: &CurtainOptions) -> Result<()> {
    if options.lock || options.uses_daemon_lock_flow() || options.initial_backoff_ms > 0 {
        bail!(
            "`veila preview` never locks the session; remove --lock and the daemon socket options"
        );
    }

    if options.preview_png.is_none() {
        help::print_preview_help();
        bail!("`veila preview` requires --preview-png=<path>");
    }

    Ok(())
}

#[cfg(test)]
mod tests;
