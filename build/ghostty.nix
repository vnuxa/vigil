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
  freetype,
  oniguruma,
  fontconfig,
  harfbuzz,
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
    deps = callPackage ./deps.nix {
      name = "ghostty-cache-${finalAttrs.version}";
      zig = zig_0_13;
    };

    nativeBuildInputs = [
      ncurses
      pandoc
      pkg-config
      removeReferencesTo
      zig_hook

      freetype
      oniguruma
      fontconfig
      harfbuzz
    ];

    zigBuildFlags = [
      "--system"
      "${finalAttrs.deps}"
      "-Dapp-runtime=none"
      "--prefix $out"
    ];
    zigCheckFlags = finalAttrs.zigBuildFlags;

    outputs = [
      "out"
    ];

    # postInstall = ''
    #         terminfo_src="$out/share/terminfo"
    #
    #   mkdir -p "$out/nix-support"
    #
    #   mkdir -p "$terminfo/share"
    #   mv "$terminfo_src" "$terminfo/share/terminfo"
    #   ln -sf "$terminfo/share/terminfo" "$terminfo_src"
    #   echo "$terminfo" >> "$out/nix-support/propagated-user-env-packages"
    #
    #   mkdir -p "$shell_integration"
    #   mv "$out/share/ghostty/shell-integration" "$shell_integration/shell-integration"
    #   ln -sf "$shell_integration/shell-integration" "$out/share/ghostty/shell-integration"
    #   echo "$shell_integration" >> "$out/nix-support/propagated-user-env-packages"
    #
    #   mv $out/share/vim/vimfiles "$vim"
    #   ln -sf "$vim" "$out/share/vim/vimfiles"
    #   echo "$vim" >> "$out/nix-support/propagated-user-env-packages"
    # '';

    doCheck = true;

    # nativeInstallCheckInputs = [
    #   versionCheckHook
    # ];

    doInstallCheck = true;
  })
