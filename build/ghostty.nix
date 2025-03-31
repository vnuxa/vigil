{
  lib,
  stdenv,
  fetchFromGitHub,
  zig_0_13,
  ncurses,
  pandoc,
  pkg-config,
  removeReferencesTo,
  versionCheckHook,
  callPackage,
}: let
  zig_hook = zig_0_13.hook.overrideAttrs {
    zig_default_flags = "-Dcpu=baseline -Doptimize=ReleaseFast --color off";
  };
in
  stdenv.mkDerivation (finalAttrs: {
    pname = "ghostty";
    version = "1.1.3";
    src = fetchFromGitHub {
      owner = "ghostty-org";
      repo = "ghostty";
      tag = "v${finalAttrs.version}";
      hash = "sha256-YHoyW+OFKxzKq4Ta/XUA9Xu0ieTfCcJo3khKpBGSnD4=";
    };
    deps = callPackage ./deps.nix {name = "ghostty-cache-${finalAttrs.version}";};

    nativeBuildInputs = [
      ncurses
      pandoc
      pkg-config
      removeReferencesTo
      zig_hook
    ];

    zigBuildFlags = [
      "-Dapp-runtime=none"
      "--prefix $out"
    ];
    zigCheckFlags = finalAttrs.zigBuildFlags;

    postInstall =
      # bash
      ''
        echo "path to out is $out"
      '';

    # doCheck = true;

    nativeInstallCheckInputs = [
      versionCheckHook
    ];

    # doInstallCheck = true;
  })
