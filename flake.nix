{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    nix-filter.url = "github:numtide/nix-filter";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    nix-filter,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      system = "x86_64-linux";
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      ghostty_package = pkgs.callPackage ./build/ghostty.nix {};
      #           pkgs.ghostty.overrideAttrs (final_attrs: previous_attrs: {
      #   appRuntime = "none";
      # });
      libraries = with pkgs; [
        ghostty_package
        wayland
        pkg-config
        libGL
        dbus
        expat
        fontconfig
        freetype
        libxkbcommon
        libclang
        rustPlatform.bindgenHook
        # openssl
      ];
      rustToolchain = pkgs.rust-bin.beta.latest.default; # beta required due to anyhow requiring cargo above 1.83
      library_path = builtins.foldl' (a: b: "${a}:${b}/lib") "${pkgs.vulkan-loader}/lib" libraries;
      rust_platform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
      vigil_package = rust_platform.buildRustPackage {
        pname = "vigil";
        version = "0.1";
        # src = ./.;
        src = nix-filter.lib.filter {
          # root = ~/Documents/Programming/astrum_unstable;
          root = self;
          include = [
            "src"
            ./src
            ./Cargo.lock
            ./Cargo.toml
          ];
        };
        buildInputs = libraries;

        nativeBuildInputs = with pkgs; [
          pkg-config
          libclang
          makeBinaryWrapper
          rustPlatform.bindgenHook
        ];

        RUSTFLAGS = map (a: "-C link-arg=${a}") [
          "-Wl,--push-state,--no-as-needed"
          "-lEGL"
          "-lwayland-client"
          "-Wl,--pop-state"
          "--release"
        ];

        postInstall =
          #bash
          ''
            wrapProgram "$out/bin/vigil"\
              --prefix CARGO_MANIFEST_DIR : "${self}"\
              --prefix LD_LIBRARY_PATH : ${
              pkgs.lib.makeLibraryPath (with pkgs; [
                libxkbcommon
                vulkan-loader
                ghostty_package
                xorg.libX11
                xorg.libXcursor
                xorg.libXi
              ])
            }
          '';

        verbose = true;
        doCheck = false;
        cargoLock = {
          lockFile = ./Cargo.lock;
          allowBuiltinFetchGit = true;
        };
      };
    in {
      packages.vigil = vigil_package;

      defaultPackage = self.packages.${system}.vigil;

      devShells.default = pkgs.mkShell {
        buildInputs = libraries;

        GHOSTTY_HEADER = "${ghostty_package}/include/ghostty.h";
        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath (with pkgs; [
          wayland
          libGL
          libxkbcommon
          ghostty_package
          libclang
        ])}";
      };
    });
}
