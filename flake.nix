{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    systems.url = "github:nix-systems/default";
  };

  outputs = inputs@{ flake-parts, crane, nixpkgs, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = import inputs.systems;

    imports = [
      inputs.flake-parts.flakeModules.easyOverlay
    ];

    perSystem = { self', pkgs, lib, ... }:
      let
        # Generate the typst package for the given nixpkgs instance.
        packageFor = pkgs:
          let
            inherit (lib)
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
                pkgs.libiconv
              ];

              nativeBuildInputs = [ pkgs.installShellFiles ];
            };

            # Derivation with just the dependencies, so we don't have to keep
            # re-building them.
            cargoArtifacts = craneLib.buildDepsOnly commonCraneArgs;
          in
          craneLib.buildPackage (commonCraneArgs // {
            inherit cargoArtifacts;

            postInstall = ''
              installManPage crates/typst-cli/artifacts/*.1
              installShellCompletion \
                crates/typst-cli/artifacts/typst.{bash,fish} \
                --zsh crates/typst-cli/artifacts/_typst
            '';

            GEN_ARTIFACTS = "artifacts";

            meta.mainProgram = "typst";
          });

        typst = packageFor pkgs;
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages = {
          default = typst;
          typst-dev = self'.packages.default;
        };

        overlayAttrs = builtins.removeAttrs self'.packages [ "default" ];

        apps.default = {
          type = "app";
          program = lib.getExe typst;
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
