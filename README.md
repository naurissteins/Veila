<h1 align=center>Veila</h1>

<div align=center>

![GitHub last commit](https://img.shields.io/github/last-commit/naurissteins/veila?style=for-the-badge&labelColor=181825&color=a6e3a1)
![GitHub repo size](https://img.shields.io/github/repo-size/naurissteins/veila?style=for-the-badge&labelColor=181825&color=d3bfe6)
![AUR Version](https://img.shields.io/aur/version/veila-bin?style=for-the-badge&labelColor=181825&color=b4befe)
![GitHub Repo stars](https://img.shields.io/github/stars/naurissteins/veila?style=for-the-badge&labelColor=181825&color=f9e2af)

**Veila is a secure, low-latency, Wayland-first screen locker written in Rust**

It is built for wlroots-style compositors like Labwc, Niri, Hyprland, Sway, MangoWC and others, and is designed around a simple rule: the secure lock path must stay small, predictable, and fast. Veila focuses on acquiring real session-lock surfaces immediately, keeping unlock authority in the daemon, and avoiding heavyweight UI stacks that add latency or complexity where it matters most.
</div>

<a href="https://github.com/user-attachments/assets/3b523594-dc4e-428e-8564-a619148973ee" target="_blank">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/3b523594-dc4e-428e-8564-a619148973ee" />
    <img alt="veila-preview2" src="https://github.com/user-attachments/assets/3b523594-dc4e-428e-8564-a619148973ee" />
  </picture>
</a>

<div align=center>
Veila aims for a modern, polished lockscreen without turning the secure path into a heavy desktop UI. Theming, widgets, and visual effects are built around that constraint instead of competing with it.

[Documentation](https://naurissteins.com/veila)
</div>

----

## 🔥 Features

- Secure Wayland session locking with `ext-session-lock-v1`
- Low-latency lock activation with a small secure curtain first and the full UI layered on top
- PAM-based authentication with unlock authority kept in the daemon
- Multi-monitor aware rendering with one secure lock surface per output
- Built-in themes plus support for user themes and small `config.toml` overrides
- Fine-grained visual customization for clock, input, layers, spacing, colors, fonts, and widget visibility
- Optional widgets like weather, battery, now playing, keyboard layout, Caps Lock, avatar, and username
- Preview tooling that renders the lockscreen directly to PNG for theme work and screenshots

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

## Install
### Arch Linux
On Arch Linux, install Veila from the AUR:

```bash
# prebuilt release package
yay -S veila-bin

# or latest git build
yay -S veila-git
```

## Start the Daemon

You can start the daemon directly:

```bash
veilad
```

Or better run it as a user service with systemd:

```bash
systemctl --user enable --now veilad.service
```

For locking directly from the CLI, use:

```bash
veila lock
```

### NixOS
On NixOS, Veila can currently be built from the flake:

```bash
nix profile install github:naurissteins/Veila#veila
```

NixOS also needs a PAM service entry so Veila can unlock with your user password:

```nix
{
  security.pam.services.veila = {};
}
```

Apply that system config with:

```bash
sudo nixos-rebuild switch
```



## Docs

For full installation, configuration, theming, and usage docs, visit:

https://naurissteins.com/veila
