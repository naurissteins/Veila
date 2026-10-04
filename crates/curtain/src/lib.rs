#![forbid(unsafe_code)]

//! Secure session-lock curtain for Veila.

mod app;
mod background;
mod cli;
mod help;
mod invocation;
mod ipc;
mod keyboard_cache;
mod options;
mod preview;
mod reload;
mod state;
mod wayland;

use anyhow::Result;
use invocation::{validate_invocation_mode, validate_preview_mode};

pub use options::{CurtainOptions, PreviewClockTime};

/// Starts the secure curtain process.
pub fn run(options: CurtainOptions) -> Result<()> {
    if options.help {
        help::print_curtain_help();
        return Ok(());
    }

    validate_invocation_mode(&options)?;

    app::run(options)
}

/// Renders the lock scene to a PNG without taking a session lock.
pub fn run_preview(options: CurtainOptions) -> Result<()> {
    if options.help {
        help::print_preview_help();
        return Ok(());
    }

    validate_preview_mode(&options)?;

    app::run(options)
}
