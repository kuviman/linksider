{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
      };

      system = "x86_64-linux";
      app = "linksider";

      shellInputs = with pkgs; [
        (rust-bin.stable.latest.default.override { extensions = [ "rust-src" ]; targets = [ "wasm32-unknown-unknown" ]; })
        clang
        trunk
        just
      ];
      appNativeBuildInputs = with pkgs; [
        pkg-config
      ];
      appBuildInputs = appRuntimeInputs ++ (with pkgs; [
        udev
        alsa-lib
        vulkan-loader
        xorg.libX11
        xorg.libXcursor
        xorg.libXi
        xorg.libXrandr # To use the x11 feature
        libxkbcommon
        wayland # To use the wayland feature
      ]);
      # TODO figure out appRuntimeInputs
      appRuntimeInputs = with pkgs; [
        vulkan-loader
        xorg.libXcursor
        xorg.libXi
        xorg.libXrandr
        wayland
      ];
    in
    {
      devShells.${system}.${app} = pkgs.mkShell {
        nativeBuildInputs = appNativeBuildInputs;
        buildInputs = shellInputs ++ appBuildInputs ++ (with pkgs; [ rust-analyzer ]);

        shellHook = ''
          export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath appRuntimeInputs}"
        '';
      };
      devShell.${system} = self.devShells.${system}.${app};

      packages.${system}.${app} = pkgs.rustPlatform.buildRustPackage {
        pname = app;
        version = "0.1.0";

        src = ./.;
        cargoSha256 = "sha256-5lRPyiqfun4eqhK2DIHPgOzUEht2TULyhzMj8LSn1L4=";

        nativeBuildInputs = appNativeBuildInputs;
        buildInputs = appBuildInputs;
        # runtimeInputs = appBuildInputs;

        postInstall = ''
          cp -r assets $out/bin/
        '';
      };
      defaultPackage.${system} = self.packages.${system}.${app};

      apps.${system}.${app} = {
        type = "app";
        program = "${self.packages.${system}.${app}}/bin/${app}";
      };
      defaultApp.${system} = self.apps.${system}.${app};

      checks.${system}.build = self.packages.${system}.${app};

      formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.nixpkgs-fmt;
    };
}
