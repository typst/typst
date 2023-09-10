{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, flake-utils, crane, nixpkgs, ... }:
    let
      # Generate the typst package for the given nixpkgs instance.
      packageFor = pkgs:
        let
          inherit (nixpkgs.lib)
            importTOML
            optionals
            sourceByRegex
            ;
          Cargo-toml = importTOML ./Cargo.toml;

          pname = "typst";
          version = Cargo-toml.workspace.package.version;

          # Crane-based Nix flake configuration.
          # Based on https://github.com/ipetkov/crane/blob/master/examples/trunk-workspace/flake.nix

          craneLib = crane.mkLib pkgs;

          # Typst files to include in the derivation.
          # Here we include Rust files, assets and tests.
          src = sourceByRegex ./. [
            "(assets|crates|tests)(/.*)?"
            ''Cargo\.(toml|lock)''
            ''build\.rs''
          ];

          # Typst derivation's args, used within crane's derivation generation
          # functions.
          commonCraneArgs = {
            inherit src pname version;

            buildInputs = optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.CoreServices
            ];

            nativeBuildInputs = [ pkgs.installShellFiles ];
          };

          # Derivation with just the dependencies, so we don't have to keep
          # re-building them.
          cargoArtifacts = craneLib.buildDepsOnly commonCraneArgs;

          typst = craneLib.buildPackage (commonCraneArgs // {
            inherit cargoArtifacts;

            postInstall = ''
              installManPage crates/typst-cli/artifacts/*.1
              installShellCompletion \
                crates/typst-cli/artifacts/typst.{bash,fish} \
                --zsh crates/typst-cli/artifacts/_typst
            '';

            GEN_ARTIFACTS = "artifacts";
          });
        in
        typst;
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      flake = {
        overlays.default = _: prev: {
          typst-dev = packageFor prev;
        };
      };

      perSystem = { pkgs, ... }:
        let
          inherit (pkgs) lib;
          typst = packageFor pkgs;
        in
        {
          packages.default = typst;

          apps.default = flake-utils.lib.mkApp {
            drv = typst;
          };

          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              cargo
            ];

            buildInputs = lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.CoreServices
              pkgs.libiconv
            ];
          };
        };
    };
}
