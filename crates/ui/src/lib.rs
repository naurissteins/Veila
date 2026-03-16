#![forbid(unsafe_code)]

//! UI scene state and rendering helpers for Kwylock.

mod shell;
mod theme;

pub use shell::{ShellAction, ShellKey, ShellState};
pub use theme::ShellTheme;

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "kwylock-ui"
}
