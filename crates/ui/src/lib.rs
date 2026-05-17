#![forbid(unsafe_code)]

//! UI scene state and rendering helpers for Veila.

mod shell;

pub use shell::{
    ShellAction, ShellAnimationUpdate, ShellKey, ShellState, ShellTheme, load_avatar,
    load_cached_avatar,
};

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "veila-ui"
}
