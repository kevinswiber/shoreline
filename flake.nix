{
  description = "Pointbreak development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      # Systems the dev shell is built for: Linux and macOS on x86_64 and arm64.
      systems = [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # cocogitto pinned to 6.5.0 to match CI (.github/workflows/*: cargo binstall
      # cocogitto@6.5.0) and mise.toml. nixpkgs ships 7.0.0, which this repo is NOT
      # ready for: cog 7 changes the release tag lifecycle the signed-tag finalizer
      # depends on (scripts/finalize-cocogitto-release-tag.sh:64) and adds native
      # scope validation the commit-msg hook still shims by hand (cog.toml:63-67).
      # Moving to cog 7 means bumping the flake, the three CI pins, removing that
      # shim, and re-validating `just release-bump-selftest` together in a
      # dedicated PR.
      mkCocogitto =
        pkgs:
        pkgs.cocogitto.overrideAttrs (_: rec {
          version = "6.5.0";
          src = pkgs.fetchFromGitHub {
            owner = "cocogitto";
            repo = "cocogitto";
            tag = version;
            hash = "sha256-aAVoPPeuJN6QPcuc3oBF93dP6U+74bAoSDw93XR01Vo=";
          };
          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit src;
            name = "cocogitto-${version}-vendor";
            hash = "sha256-yDpZHkRKsWXXHuSKnzhGrjsFLUFZEpC23tcU3FeUZK8=";
          };
          # 6.5.0's completion subcommand differs from 7.x; completions aren't
          # needed here, so skip the postInstall that generates them.
          postInstall = "";
        });
    in
    {
      # `nix fmt` formats the flake with the canonical RFC-166 formatter.
      formatter = forEachSystem (pkgs: pkgs.nixfmt);

      devShells = forEachSystem (
        pkgs:
        let
          cocogitto = mkCocogitto pkgs;
        in
        {
          default = pkgs.mkShell {
            # Everything on PATH inside `nix develop`.
            packages = with pkgs; [
              # --- Rust ---
              # rustup (not a fixed rustc) because the Justfile/CI use BOTH
              # `cargo +stable` (build/test/clippy) and `cargo +nightly` (rustfmt,
              # which relies on unstable_features). Pinning a single rustc in Nix
              # can't serve both channels; rustup keeps rust-toolchain.toml as the
              # source of truth for stable and installs each channel on demand (see
              # RUSTUP_AUTO_INSTALL in the shellHook).
              rustup

              # --- Dev tooling (mirrors mise.toml [tools]) ---
              just
              cargo-nextest
              cargo-edit
              cocogitto # `cog`, pinned to 6.5.0 for CI parity — see mkCocogitto above
              gh
              jq
              nodejs_22

              # --- Native build deps ---
              # libsqlite3-sys / rusqlite / zstd / lmdb-master3-sys all compile
              # bundled C, so cargo needs a working C toolchain and pkg-config.
              # NixOS has no global cc; mkShell's stdenv provides one, and these make
              # it explicit.
              pkg-config
              git # used by build.rs (identity capture) and by cog hooks
            ];

            shellHook = ''
              # Let `cargo +stable` and `cargo +nightly` install their toolchain on
              # first use instead of erroring. stable comes from rust-toolchain.toml;
              # nightly (needed by `just fmt`) is fetched the first time it's invoked.
              export RUSTUP_AUTO_INSTALL=1

              # Replicate mise's `[env] _.path`: prefer freshly-built binaries.
              # Guarded so re-sourcing the hook doesn't stack duplicate entries.
              case ":$PATH:" in
                *":$PWD/target/debug:"*) ;;
                *) export PATH="$PWD/target/release:$PWD/target/debug:$PATH" ;;
              esac

              # Install cocogitto's commit-msg / pre-push hooks once (mise did this via
              # a postinstall step). Idempotent: only runs when the hook is missing.
              if [ -d .git ] && [ ! -f .git/hooks/commit-msg ]; then
                cog install-hook --all >/dev/null 2>&1 \
                  && echo "pointbreak: installed cocogitto git hooks"
              fi

              echo "pointbreak dev shell — rustup $(rustup --version 2>/dev/null | awk '{print $2}'), just, nextest, cog, node $(node --version)"
            '';
          };
        }
      );

      # `nix flake check` builds every derivation under `checks`. This one realises
      # the pinned cocogitto (proving the from-source pin still compiles on a clean
      # machine) and asserts the version-critical tools resolve, so a broken pin or
      # version drift fails the flake rather than only surfacing in a live shell.
      checks = forEachSystem (
        pkgs:
        let
          cocogitto = mkCocogitto pkgs;
        in
        {
          devshell-tools =
            pkgs.runCommand "devshell-tools-check"
              {
                nativeBuildInputs = [
                  cocogitto
                  pkgs.nodejs_22
                  pkgs.just
                  pkgs.cargo-nextest
                ];
              }
              ''
                # rustup is intentionally excluded: it insists on a writable HOME
                # to create ~/.rustup, which the build sandbox denies. Its presence
                # is already covered by evaluating the devShell.
                cog --version | grep -qw 6.5.0
                node --version | grep -q '^v22\.'
                just --version >/dev/null
                cargo-nextest nextest --version >/dev/null
                touch "$out"
              '';
        }
      );
    };
}
