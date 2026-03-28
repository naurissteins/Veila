# Veila

Veila is a secure, low-latency, Wayland-first screen locker written in Rust.

It is built for wlroots-style compositors like Hyprland, Sway, MangoWC and others, and is designed around a simple rule: the secure lock path must stay small, predictable, and fast. Veila focuses on acquiring real session-lock surfaces immediately, keeping unlock authority in the daemon, and avoiding heavyweight UI stacks that add latency or complexity where it matters most.

Veila is not a fullscreen desktop window pretending to be a lockscreen. It is a compositor-aware lock application that uses the Wayland session-lock protocol and a dedicated daemon to manage lock, auth, and unlock state.


<img width="2006" height="1166" alt="veila-preview-default" src="https://github.com/user-attachments/assets/60c67e91-0d1b-4f29-b370-d390a33f39a4" />


## Why Veila Is Secure

Veila is built around a security-first model:

- it uses `ext-session-lock-v1` for the real lock surface
- it relies on `logind` as the source of truth for lock and unlock state
- it uses PAM for authentication instead of custom auth logic
- unlock decisions stay in the daemon, not in the UI
- the password UI runs inside the secure session-lock path, not in a separate untrusted window
- IPC is explicit and typed instead of ad hoc

## Why Veila Is Fast

Veila is designed to avoid visible lock activation gaps and unnecessary work on the critical path.

- the secure curtain acquires lock surfaces first
- expensive work is pushed out of the activation path where possible
- wallpapers are cached and prewarmed
- text layout and icon rasterization are cached
- optional widgets such as weather, now playing, and battery render from cached daemon-side snapshots
- the lock path avoids network fetches, heavy toolkit startup, and unnecessary process churn

Release builds are the target for user experience. The project is optimized around real lock responsiveness, not around making debug builds look artificially fast.

## No CSS or Web UI Engine

Veila does not use a CSS-driven web UI engine, embedded browser stack, or heavyweight desktop toolkit for the lockscreen.

That is an intentional engineering decision. The lock surface is rendered by project-owned Rust code so the secure path stays:

- smaller in dependency surface
- easier to reason about
- more predictable in performance
- less dependent on a generic theming/runtime layer that was not designed for a security-sensitive lockscreen

Theming and visual customization are handled through typed `config.toml` settings instead of a CSS runtime.

## Technology Stack

Veila currently uses:

- Rust stable
- Smithay Client Toolkit (SCTK)
- Wayland session-lock protocols
- `logind`
- PAM
- `tiny-skia` for shared 2D software rendering primitives
- `cosmic-text` for text shaping and rasterization
- project-owned IPC, UI, and renderer code
