<h1 align=center>Veila</h1>

<div align=center>

![GitHub last commit](https://img.shields.io/github/last-commit/naurissteins/veila?style=for-the-badge&labelColor=181825&color=a6e3a1)
![GitHub repo size](https://img.shields.io/github/repo-size/naurissteins/veila?style=for-the-badge&labelColor=181825&color=d3bfe6)
![AUR Version](https://img.shields.io/aur/version/veila-bin?style=for-the-badge&labelColor=181825&color=b4befe)
![GitHub Repo stars](https://img.shields.io/github/stars/naurissteins/veila?style=for-the-badge&labelColor=181825&color=f9e2af)

Veila is built for wlroots-style compositors like labwc, Niri, Hyprland, Sway, MangoWC and others that support the Wayland ext-session-lock-v1 protocol. Its main goal is to provide a secure, fast and elegant lock screen without relying on heavyweight UI stacks.
</div>

<a href="https://github.com/user-attachments/assets/3b523594-dc4e-428e-8564-a619148973ee" target="_blank">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/3b523594-dc4e-428e-8564-a619148973ee" />
    <img alt="veila-preview2" src="https://github.com/user-attachments/assets/3b523594-dc4e-428e-8564-a619148973ee" />
  </picture>
</a>

<div align=center>

[Documentation](https://naurissteins.com/veila)

**[Arch Linux](https://naurissteins.com/veila/docs/installation/arch-linux)** | **[NixOS](https://naurissteins.com/veila/docs/installation/nixos)** | **[Ubuntu](https://naurissteins.com/veila/docs/installation/debian-ubuntu)** | **[Fedora](https://naurissteins.com/veila/docs/installation/fedora)**

</div>

----

## 🔥 Features

- Fast and secure locking with a clean polished UI
- Multi-monitor support
- Simple setup with `config.toml`
- Built-in themes plus support custom themes
- Flexible styling for the clock, password field, colors, fonts and more
- Widgets: weather, battery, now playing, keyboard layout, Caps Lock, avatar and username
- Color-only backgrounds, wallpaper backgrounds and per-monitor background overrides
- Preview mode, generate lockscreen to a PNG for screenshots and theme work
- Lightweight design without a heavy desktop UI toolkit

## Install
### Arch Linux
On Arch Linux, install Veila from the AUR:

```bash
# prebuilt release package (recommended)
yay -S veila-bin

# or latest git build
yay -S veila-git
```

### First launch

Start the daemon:

```bash
veilad
```

Or better run it as a user service with systemd:

```bash
systemctl --user enable --now veilad.service
```

Lock the screen:

```bash
veila lock
```

### NixOS

**Flake installation:**

```nix
{
  inputs.veila.url = "github:naurissteins/Veila";

  outputs = { nixpkgs, veila, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        veila.nixosModules.default
        {
          programs.veila.enable = true;
        }
      ];
    };
  };
}
```

The module installs `veila`, `veilad` and `veila-curtain` and configures the required PAM service.

**Install directly:**

```bash
nix profile install github:naurissteins/Veila#veila
```

Add PAM service:

```nix
{
  environment.systemPackages = [
    inputs.veila.packages.${pkgs.system}.default
  ];

  security.pam.services.veila = {};
}
```

## Docs

For full installation, configuration, theming and more, visit:

https://naurissteins.com/veila
