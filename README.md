# Veila

Veila is a secure, low-latency, Wayland-first screen locker written in Rust.

It is built for wlroots-style compositors like Labwc, Niri, Hyprland, Sway, MangoWC and others, and is designed around a simple rule: the secure lock path must stay small, predictable, and fast. Veila focuses on acquiring real session-lock surfaces immediately, keeping unlock authority in the daemon, and avoiding heavyweight UI stacks that add latency or complexity where it matters most.

<img width="2006" height="1166" alt="veila-preview2" src="https://github.com/user-attachments/assets/1cce249d-bf3f-4f0f-8a87-f6448ee21d24" />

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
