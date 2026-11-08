use anyhow::{Result, bail};

use crate::{CurtainOptions, help};

pub(super) fn validate_invocation_mode(options: &CurtainOptions) -> Result<()> {
    if options.ipc_fd.is_some() != options.lock_fd.is_some() {
        bail!("supervised curtain requires both inherited descriptors");
    }
    if let (Some(ipc), Some(lock)) = (options.ipc_fd, options.lock_fd)
        && (ipc < 3 || lock < 3 || ipc == lock || !options.lock || options.config_path.is_some())
    {
        bail!("invalid supervised curtain invocation");
    }
    if options.preview_png.is_some() || options.lock {
        return Ok(());
    }

    help::print_curtain_help();
    bail!(
        "refusing to start a real lock session from a plain `veila __curtain` launch; use `veila lock`, or pass `--lock` if you really want a direct curtain test"
    );
}

pub(super) fn validate_preview_mode(options: &CurtainOptions) -> Result<()> {
    if options.lock
        || options.initial_backoff_ms > 0
        || options.ipc_fd.is_some()
        || options.lock_fd.is_some()
    {
        bail!(
            "`veila preview` never locks the session; remove --lock and the inherited supervisor options"
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
