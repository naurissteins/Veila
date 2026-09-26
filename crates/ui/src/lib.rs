#![forbid(unsafe_code)]

//! UI scene state and rendering helpers for Veila.

pub mod background;
mod shell;

pub use shell::{
    ShellAction, ShellAnimationUpdate, ShellKey, ShellState, ShellTheme, has_avatar_candidate,
    load_avatar, load_cached_avatar,
};
