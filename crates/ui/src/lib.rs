#![forbid(unsafe_code)]

//! UI scene state and rendering helpers for Veila.

pub mod background;
mod shell;

pub use shell::{
    AvatarTheme, CapsLockTheme, ClockTheme, DateTheme, EyeTheme, InputTheme, NowPlayingTheme,
    PlaceholderTheme, RevealTheme, ShellAction, ShellAnimationUpdate, ShellKey, ShellState,
    ShellTheme, StatusTheme, UsernameTheme, WidgetDamage, WidgetKind, WidgetRegions,
    has_avatar_candidate, load_avatar, load_cached_avatar,
};
