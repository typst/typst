{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";

    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-manifest = {
      url = "https://static.rust-lang.org/dist/channel-rust-1.91.0.toml";
      flake = false;
    };
  };

  outputs =
    inputs@{
      flake-parts,
      crane,
      nixpkgs,
      fenix,
      rust-manifest,
      self,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      imports = [
        inputs.flake-parts.flakeModules.easyOverlay
      ];

      perSystem =
        {
          self',
          pkgs,
          lib,
          system,
          ...
        }:
        let
          cargoToml = lib.importTOML ./Cargo.toml;

          pname = "typst";
          version = cargoToml.workspace.package.version;

          rust-toolchain = fenix.packages.${system}.fromManifestFile rust-manifest;

          # Crane-based Nix flake configuration.
          # Based on https://github.com/ipetkov/crane/blob/master/examples/trunk-workspace/flake.nix
          craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain.defaultToolchain;

          # Typst files to include in the derivation.
          # Here we include Rust files, docs and tests.
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./rustfmt.toml
              ./crates
              ./docs
              ./tests
            ];
          };

          # Typst derivation's args, used within crane's derivation generation
          # functions.
          commonCraneArgs = {
            inherit src pname version;

            buildInputs = [
              pkgs.openssl
            ];

            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.openssl.dev
            ];

            env = {
              RUSTFLAGS =
                if pkgs.stdenv.hostPlatform.rust.rustcTargetSpec == "x86_64-unknown-linux-gnu" then
                  # Upstream defaults to lld on x86_64-unknown-linux-gnu, we need to use the system linker
                  "-Clinker-features=-lld -Clink-self-contained=-linker"
                else
                  null;
            };
          };

          # Derivation with just the dependencies, so we don't have to keep
          # re-building them.
          cargoArtifacts = craneLib.buildDepsOnly commonCraneArgs;

          typst = craneLib.buildPackage (
            commonCraneArgs
            // {
              inherit cargoArtifacts;

              nativeBuildInputs = commonCraneArgs.nativeBuildInputs ++ [
                pkgs.installShellFiles
              ];

              postInstall = ''
                installManPage crates/typst-cli/artifacts/*.1
                installShellCompletion \
                  crates/typst-cli/artifacts/typst.{bash,fish} \
                  --zsh crates/typst-cli/artifacts/_typst
              '';

              GEN_ARTIFACTS = "artifacts";
              TYPST_VERSION = cargoToml.workspace.package.version;
              TYPST_COMMIT_SHA = self.shortRev or "dirty";

              meta.mainProgram = "typst";
            }
          );
        in
        {
          formatter = pkgs.nixfmt-tree;

          packages = {
            default = typst;
            typst-dev = self'.packages.default;
          };

          overlayAttrs = builtins.removeAttrs self'.packages [ "default" ];

          apps.default = {
            type = "app";
            program = lib.getExe typst;
          };

          checks = {
            typst-fmt = craneLib.cargoFmt commonCraneArgs;
            typst-clippy = craneLib.cargoClippy (
              commonCraneArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--workspace -- --deny warnings";
              }
            );
            typst-test = craneLib.cargoTest (
              commonCraneArgs
              // {
                inherit cargoArtifacts;
                cargoTestExtraArgs = "--workspace";
              }
            );
          };

          devShells.default = craneLib.devShell {
            checks = self'.checks;
            inputsFrom = [ typst ];

            buildInputs = [
              rust-toolchain.rust-analyzer
              rust-toolchain.rust-src
            ];

            RUST_SRC_PATH = "${rust-toolchain.rust-src}/lib/rustlib/src/rust/library";

            packages = [
              # A script for quickly running tests.
              # See https://github.com/typst/typst/blob/main/tests/README.md#making-an-alias
              (pkgs.writeShellScriptBin "testit" ''
                cargo test --workspace --test tests -- "$@"
              '')
            ];
          };
        };
    };
}
