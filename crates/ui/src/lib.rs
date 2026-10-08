#![forbid(unsafe_code)]

//! UI scene state and rendering helpers for Veila.

pub mod background;
mod shell;

pub use shell::{
    CapsLockTheme, EyeTheme, InputTheme, PlaceholderTheme, RevealTheme, ShellAction,
    ShellAnimationUpdate, ShellKey, ShellState, ShellTheme, StatusTheme, WidgetDamage, WidgetKind,
    WidgetRegions, has_avatar_candidate, load_avatar, load_cached_avatar,
};
