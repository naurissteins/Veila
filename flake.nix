{
  description = "Veila - Secure, elegant, and fast Wayland screen locker";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: import nixpkgs { inherit system; };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        rec {
          veila = pkgs.rustPlatform.buildRustPackage {
            pname = "veila";
            version = "0.4.3";

            src = self;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            cargoBuildFlags = [ "--workspace" ];
            cargoCheckFlags = [ "--workspace" ];

            nativeBuildInputs = with pkgs; [
              makeWrapper
              pkg-config
            ];

            buildInputs = with pkgs; [
              libxkbcommon
              pam
              wayland
            ];

            installPhase = ''
              runHook preInstall

              veila_bin="$(find target -type f -path '*/release/veila' -print -quit)"
              veilad_bin="$(find target -type f -path '*/release/veilad' -print -quit)"
              curtain_bin="$(find target -type f -path '*/release/veila-curtain' -print -quit)"

              if [ -z "$veila_bin" ] || [ -z "$veilad_bin" ] || [ -z "$curtain_bin" ]; then
                echo "failed to find release binaries under target/"
                find target -maxdepth 4 -type f -perm -0100 -print
                exit 1
              fi

              install -Dm755 "$veila_bin" "$out/bin/veila"
              install -Dm755 "$veilad_bin" "$out/bin/veilad"
              install -Dm755 "$curtain_bin" "$out/bin/veila-curtain"
              install -Dm644 docs/man/veila.1 "$out/share/man/man1/veila.1"

              mkdir -p "$out/share/veila"
              cp -R assets/fonts "$out/share/veila/"
              cp -R assets/icons "$out/share/veila/"
              cp -R assets/systemd "$out/share/veila/"
              cp -R assets/themes "$out/share/veila/"

              wrapProgram "$out/bin/veila-curtain" \
                --set VEILA_ASSET_DIR "$out/share/veila"

              wrapProgram "$out/bin/veila" \
                --set VEILA_ASSET_DIR "$out/share/veila"

              wrapProgram "$out/bin/veilad" \
                --set VEILA_ASSET_DIR "$out/share/veila" \
                --set VEILA_CURTAIN_BIN "$out/bin/veila-curtain"

              runHook postInstall
            '';

            meta = {
              description = "Secure, elegant, and fast Wayland screen locker";
              homepage = "https://naurissteins.com/veila";
              license = pkgs.lib.licenses.gpl3Plus;
              mainProgram = "veila";
              platforms = pkgs.lib.platforms.linux;
            };
          };

          default = veila;
        }
      );

      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.veila;
          package = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        in
        {
          options.programs.veila = {
            enable = lib.mkEnableOption "Veila screen locker";

            package = lib.mkOption {
              type = lib.types.package;
              default = package;
              defaultText = lib.literalExpression "inputs.veila.packages.${pkgs.system}.default";
              description = "Veila package to install.";
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];
            security.pam.services.veila = { };
          };
        };

      homeModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.veila;
          tomlFormat = pkgs.formats.toml { };
        in
        {
          options.programs.veila = {
            enable = lib.mkEnableOption "Veila screen locker";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              defaultText = lib.literalExpression "inputs.veila.packages.\${pkgs.system}.default";
              description = "Veila package to install.";
            };

            settings = lib.mkOption {
              type = tomlFormat.type;
              default = { };
              example = lib.literalExpression ''{ theme = "santorini"; }'';
              description = "Written verbatim as TOML to ~/.config/veila/config.toml.";
            };

            service.enable = lib.mkEnableOption "the veilad daemon as a systemd user service";

            idle = {
              enable = lib.mkEnableOption "the veila idle/sleep auto-lock helper";

              lockAfter = lib.mkOption {
                type = lib.types.ints.positive;
                default = 300;
                description = "Seconds of inactivity before locking.";
              };

              lockBeforeSleep = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Also lock before the system goes to sleep.";
              };
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ cfg.package ];

            xdg.configFile."veila/config.toml" = lib.mkIf (cfg.settings != { }) {
              source = tomlFormat.generate "veila-config.toml" cfg.settings;
            };

            systemd.user.services.veilad = lib.mkIf cfg.service.enable {
              Unit = {
                Description = "Veila screen locker daemon";
                After = [ "graphical-session.target" ];
                PartOf = [ "graphical-session.target" ];
              };
              Service = {
                Type = "simple";
                ExecStart = "${cfg.package}/bin/veilad";
                Restart = "on-failure";
                RestartSec = 2;
                PassEnvironment = "WAYLAND_DISPLAY XDG_SESSION_ID XDG_SESSION_TYPE XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE SWAYSOCK NIRI_SOCKET";
              };
              Install.WantedBy = [ "graphical-session.target" ];
            };

            systemd.user.services.veila-idle = lib.mkIf cfg.idle.enable {
              Unit = {
                Description = "Veila idle and sleep lock monitor";
                After = [
                  "graphical-session.target"
                  "veilad.service"
                ];
                PartOf = [ "graphical-session.target" ];
              };
              Service = {
                Type = "simple";
                ExecStart =
                  "${cfg.package}/bin/veila idle --lock-after=${toString cfg.idle.lockAfter}"
                  + lib.optionalString cfg.idle.lockBeforeSleep " --lock-before-sleep";
                Restart = "on-failure";
                RestartSec = 2;
                PassEnvironment = "WAYLAND_DISPLAY XDG_SESSION_ID XDG_SESSION_TYPE XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE SWAYSOCK NIRI_SOCKET";
              };
              Install.WantedBy = [ "graphical-session.target" ];
            };
          };
        };

      apps = forAllSystems (
        system:
        let
          package = self.packages.${system}.veila;
        in
        {
          veila = {
            type = "app";
            program = "${package}/bin/veila";
          };

          veilad = {
            type = "app";
            program = "${package}/bin/veilad";
          };

          veila-curtain = {
            type = "app";
            program = "${package}/bin/veila-curtain";
          };

          default = self.apps.${system}.veila;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              cargo-deny
              libxkbcommon
              pam
              pkg-config
              rustc
              rustfmt
              wayland
            ];
          };
        }
      );
    };
}
