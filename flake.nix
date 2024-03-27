{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    systems.url = "github:nix-systems/default";
  };

  outputs = inputs@{ flake-parts, crane, nixpkgs, self, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = import inputs.systems;

    imports = [
      inputs.flake-parts.flakeModules.easyOverlay
    ];

    perSystem = { self', pkgs, lib, ... }:
      let
        cargo_version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
        typst_revision = self.shortRev or "dirty";
        typst_version = "${cargo_version} (${typst_revision})";

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
            # Here we include Rust files, docs and tests.
            src = sourceByRegex ./. [
              "(docs|crates|tests)(/.*)?"
              ''Cargo\.(toml|lock)''
              ''build\.rs''
            ];

            # Typst derivation's args, used within crane's derivation generation
            # functions.
            commonCraneArgs = {
              inherit src pname version;

              buildInputs = (optionals pkgs.stdenv.isDarwin [
                pkgs.darwin.apple_sdk.frameworks.CoreServices
                pkgs.libiconv
              ]) ++ [
                pkgs.openssl
              ];

              nativeBuildInputs = [
                pkgs.installShellFiles
                pkgs.pkg-config
                pkgs.openssl.dev
              ];
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
            TYPST_VERSION = typst_version;

            meta = {
              description = "Typst, a markup-based typesetting system that is powerful and easy to learn";
              homepage = "https://typst.app";
              license = lib.licenses.asl20;
              mainProgram = "typst";
            };
          });

        typst = packageFor pkgs;
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages = {
          default = typst;
          typst-dev = self'.packages.default;
          oci-typst = pkgs.dockerTools.buildLayeredImage {
            name = "typst";
            tag = cargo_version;
            contents = [ typst ];
            config = {
              Entrypoint = [ "typst" ];
              Labels = {
                "org.opencontainers.image.description" = self'.packages.default.meta.description;
                "org.opencontainers.image.licenses" = self'.packages.default.meta.license.spdxId;
                "org.opencontainers.image.revision" = typst_revision;
                "org.opencontainers.image.source" = "https://github.com/typst/typst";
                "org.opencontainers.image.title" = "Typst";
                "org.opencontainers.image.url" = self'.packages.default.meta.homepage;
                "org.opencontainers.image.version" = cargo_version;
              };
            };
          };
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

          buildInputs = (lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.CoreServices
            pkgs.libiconv
          ]) ++ [
            pkgs.openssl
          ];

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.openssl.dev
          ];
        };
      };
  };
}
