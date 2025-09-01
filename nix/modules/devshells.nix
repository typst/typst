{ inputs, ... }:
{
  perSystem =
    {
      pkgs,
      config,
      system,
      ...
    }:

    let
      rust-toolchain = inputs.fenix.packages.${system}.fromManifestFile inputs.rust-manifest;

      # Crane-based Nix flake configuration.
      # Based on https://github.com/ipetkov/crane/blob/master/examples/trunk-workspace/flake.nix
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rust-toolchain.defaultToolchain;
    in
    {
      devShells.default = craneLib.devShell {
        checks = config.packages.typst.checks;
        inputsFrom = [ config.packages.typst ];

        buildInputs = [
          rust-toolchain.rust-analyzer
          rust-toolchain.rust-src
        ];

        env.RUST_SRC_PATH = "${rust-toolchain.rust-src}/lib/rustlib/src/rust/library";

        packages = [
          # A script for quickly running tests.
          # See https://github.com/typst/typst/blob/main/tests/README.md#making-an-alias
          (pkgs.writeShellScriptBin "testit" ''
            cargo test --workspace --test tests -- "$@"
          '')
        ];
      };
    };
}
