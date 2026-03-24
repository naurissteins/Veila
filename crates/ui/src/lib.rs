#![forbid(unsafe_code)]

//! UI scene state and rendering helpers for Veila.

mod shell;

pub use shell::{ShellAction, ShellKey, ShellState, ShellTheme};

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "veila-ui"
}
