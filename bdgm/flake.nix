{
  description = "BDGM utils";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      naersk,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk' = pkgs.callPackage naersk { };
        guiBuildInputs = with pkgs; [
          wayland
          libxkbcommon
          libGL
          expat
          fontconfig
          freetype
          openssl
          libX11
          libXcursor
          libXi
          libXrandr
        ];
        mkPackage =
          name: bin-name:
          naersk'.buildPackage {
            src = ./.;
            pname = name;
            cargoBuildOptions =
              x:
              x
              ++ [
                "-p"
                name
              ];
            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = guiBuildInputs;
            meta.mainProgram = bin-name;
          };
      in
      {
        packages = {
          play = mkPackage "bdgm-play" "bdgm-play";
          build = mkPackage "bdgm-build" "bdgm-build";
          default = self.packages.${system}.play;
        };
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
          ];

          buildInputs = guiBuildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath guiBuildInputs}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
          '';
        };
      }
    );
}
