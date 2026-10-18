# SPDX-FileCopyrightText: 2021 László Vaskó <opensource@vlaci.email>
#
# SPDX-License-Identifier: MPL-2.0

{
  description = "Transparent file encryption in git with age";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    crane.url = "github:ipetkov/crane";

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      advisory-db,
      git-hooks,
      ...
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      nixpkgsFor = forAllSystems (
        system:
        import nixpkgs {
          inherit system;
          overlays = [ self.overlays.default ];
        }
      );

      pre-commit-check = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
        in
        git-hooks.lib.${system}.run {
          src = ./.;
          package = pkgs.prek;
          hooks = {
            check-added-large-files.enable = true;
            end-of-file-fixer.enable = true;
            check-yaml.enable = true;
            check-toml.enable = true;
            nixfmt.enable = true;
            statix.enable = true;
            deadnix.enable = true;
            cargo-check.enable = true;
            clippy = {
              enable = true;
              packageOverrides.cargo = pkgs.cargo;
              packageOverrides.clippy = pkgs.clippy;
              settings = {
                denyWarnings = true;
                offline = false;
                extraArgs = "--all-targets";
              };
            };
            rustfmt.enable = true;
            reuse.enable = true;
          };
        }
      );
    in
    {
      overlays.default =
        final: _prev:
        let
          inherit (final) lib stdenv;
          craneLib = crane.mkLib final;
          commonArgs = {
            src = craneLib.cleanCargoSource ./.;
            strictDeps = true;
            # git: the e2e tests and the git.rs unit tests shell out to it
            nativeBuildInputs = [
              final.pkg-config
              final.git
            ];
            buildInputs = [
              final.libgit2
              final.zlib
            ];
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          git-agecrypt = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              # Tests run once, in passthru.tests.test.
              doCheck = false;
              passthru.tests = {
                clippy = craneLib.cargoClippy (
                  commonArgs
                  // {
                    inherit cargoArtifacts;
                    cargoClippyExtraArgs = "--all-targets -- --deny warnings";
                  }
                );

                fmt = craneLib.cargoFmt commonArgs;

                test =
                  let
                    testCommand =
                      if stdenv.hostPlatform.system == "aarch64-linux" then craneLib.cargoTest else craneLib.cargoNextest;
                  in
                  testCommand (commonArgs // { inherit cargoArtifacts; });

                audit = craneLib.cargoAudit (
                  commonArgs
                  // {
                    inherit advisory-db;
                    # RUSTSEC-2023-0071 (rsa, Marvin timing attack, no fixed release) comes in via age's
                    # `ssh` feature. Exploiting it requires timing many chosen-ciphertext decryptions;
                    # the filter only decrypts locally with the user's own key.
                    cargoAuditExtraArgs = "--ignore yanked --ignore RUSTSEC-2023-0071";
                  }
                );
              }
              // lib.optionalAttrs (stdenv.hostPlatform.system == "x86_64-linux") {
                test-coverage =
                  (craneLib.cargoNextest (
                    commonArgs
                    // {
                      inherit cargoArtifacts;
                      env = {
                        inherit (final.cargo-llvm-cov) LLVM_COV LLVM_PROFDATA;
                      };
                      cargoLlvmCovExtraArgs = "--ignore-filename-regex /nix/store --codecov --output-path $out/coverage.codecov";
                      withLlvmCov = true;
                    }
                  )).overrideAttrs
                    (super: {
                      pname = "${super.pname}-coverage";
                    });
              };
            }
          );
        };

      # Deprecated alias kept for existing consumers.
      overlay = self.overlays.default;

      checks = forAllSystems (
        system:
        let
          inherit (nixpkgsFor.${system}) git-agecrypt;
        in
        git-agecrypt.passthru.tests // { inherit git-agecrypt; }
      );

      packages = forAllSystems (
        system:
        let
          inherit (nixpkgsFor.${system}) git-agecrypt;
        in
        {
          inherit git-agecrypt;
          default = git-agecrypt;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
          # RustRover wants a rustup-like toolchain: cargo, rustc, cc in one bin/ and stdlib sources at
          # `$(rustc --print sysroot)/lib/rustlib/src/rust/library`. It copies those sources into its
          # cache preserving modes, so they must not live in the read-only store.
          rustToolchainBins = pkgs.symlinkJoin {
            name = "rust-toolchain-bins";
            paths = with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              stdenv.cc
            ];
          };
          setupRustToolchain = pkgs.writeShellScript "setup-rust-toolchain" ''
            set -eu
            dir=$1
            [ "$(cat "$dir/.stamp" 2>/dev/null)" = "$0" ] && exit 0
            if [ -L "$dir" ]; then rm "$dir"; else rm -rf "$dir"; fi
            mkdir -p "$dir/bin" "$dir/lib/rustlib/src/rust"
            ln -s ${rustToolchainBins}/bin/* "$dir/bin/"
            rm "$dir/bin/rustc"
            printf '#!${pkgs.runtimeShell}\nexec ${pkgs.rustc}/bin/rustc --sysroot "%s" "$@"\n' "$dir" > "$dir/bin/rustc"
            chmod +x "$dir/bin/rustc"
            ln -s ${pkgs.rustc.unwrapped}/lib/rustlib/* "$dir/lib/rustlib/"
            cp -r --no-preserve=mode ${pkgs.rustPlatform.rustLibSrc} "$dir/lib/rustlib/src/rust/library"
            echo "$0" > "$dir/.stamp"
          '';
        in
        {
          default = pkgs.mkShell {
            shellHook = pre-commit-check.${system}.shellHook + ''
              if [ -d .direnv ]; then ${setupRustToolchain} "$PWD/.direnv/rust-toolchain"; fi
            '';
            inputsFrom = [ pkgs.git-agecrypt ];
            packages =
              with pkgs;
              [
                cargo
                rustc
                cargo-llvm-cov
                cargo-nextest
                cargo-watch
                lldb
              ]
              ++ lib.optional stdenv.hostPlatform.isLinux gdb
              ++ pre-commit-check.${system}.enabledPackages;
            env = {
              inherit (pkgs.cargo-llvm-cov) LLVM_COV LLVM_PROFDATA;
            };
          };
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
          inherit (pre-commit-check.${system}.config) package configFile;
          script = ''
            ${pkgs.lib.getExe package} run --all-files --config ${configFile}
          '';
        in
        pkgs.writeShellScriptBin "pre-commit-run" script
      );
    };
}
