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
            version = "0.1.3";

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

              veilad_bin="$(find target -type f -path '*/release/veilad' -print -quit)"
              curtain_bin="$(find target -type f -path '*/release/veila-curtain' -print -quit)"

              if [ -z "$veilad_bin" ] || [ -z "$curtain_bin" ]; then
                echo "failed to find release binaries under target/"
                find target -maxdepth 4 -type f -perm -0100 -print
                exit 1
              fi

              install -Dm755 "$veilad_bin" "$out/bin/veilad"
              install -Dm755 "$curtain_bin" "$out/bin/veila-curtain"

              mkdir -p "$out/share/veila"
              cp -R assets/bg "$out/share/veila/"
              cp -R assets/fonts "$out/share/veila/"
              cp -R assets/icons "$out/share/veila/"
              cp -R assets/systemd "$out/share/veila/"
              cp -R assets/themes "$out/share/veila/"

              wrapProgram "$out/bin/veila-curtain" \
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
              mainProgram = "veilad";
              platforms = pkgs.lib.platforms.linux;
            };
          };

          default = veila;
        }
      );

      apps = forAllSystems (
        system:
        let
          package = self.packages.${system}.veila;
        in
        {
          veilad = {
            type = "app";
            program = "${package}/bin/veilad";
          };

          veila-curtain = {
            type = "app";
            program = "${package}/bin/veila-curtain";
          };

          default = self.apps.${system}.veilad;
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
