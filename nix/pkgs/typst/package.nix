{
  inputs,
  lib,
  system,
  pkgs,
  openssl,
  pkg-config,
  installShellFiles,
  ...
}:
let
  cargoToml = lib.importTOML ../../../Cargo.toml;

  pname = "typst";
  version = cargoToml.workspace.package.version;

  rust-toolchain = inputs.fenix.packages.${system}.fromManifestFile inputs.rust-manifest;

  # Crane-based Nix flake configuration.
  # Based on https://github.com/ipetkov/crane/blob/master/examples/trunk-workspace/flake.nix
  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rust-toolchain.defaultToolchain;

  # Typst files to include in the derivation.
  # Here we include Rust files, docs and tests.
  src = lib.fileset.toSource {
    root = ../../../.;
    fileset = lib.fileset.unions [
      ../../../Cargo.toml
      ../../../Cargo.lock
      ../../../rustfmt.toml
      ../../../crates
      ../../../docs
      ../../../tests
    ];
  };

  # Typst derivation's args, used within crane's derivation generation
  # functions.
  commonCraneArgs = {
    inherit src pname version;

    buildInputs = [
      openssl
    ];

    nativeBuildInputs = [
      pkg-config
      openssl.dev
    ];
  };

  # Derivation with just the dependencies, so we don't have to keep
  # re-building them.
  cargoArtifacts = craneLib.buildDepsOnly commonCraneArgs;
in
craneLib.buildPackage (
  commonCraneArgs
  // {
    inherit cargoArtifacts;

    nativeBuildInputs = commonCraneArgs.nativeBuildInputs ++ [
      installShellFiles
    ];

    postInstall = ''
      installManPage crates/typst-cli/artifacts/*.1
      installShellCompletion \
        crates/typst-cli/artifacts/typst.{bash,fish} \
        --zsh crates/typst-cli/artifacts/_typst
    '';

    env = {
      GEN_ARTIFACTS = "artifacts";
      TYPST_VERSION =
        let
          inherit (cargoToml.workspace.package) version;
          rev = inputs.self.shortRev or "dirty";
        in
        "${version} (${rev})";
    };

    passthru = {
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
    };

    meta.mainProgram = "typst";
  }
)
