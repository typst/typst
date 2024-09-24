{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";

    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs@{ flake-parts, crane, nixpkgs, self, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = import inputs.systems;

    imports = [
      inputs.flake-parts.flakeModules.easyOverlay
    ];

    perSystem = { self', pkgs, lib, ... }:
      let
        cargoToml = lib.importTOML ./Cargo.toml;

        pname = "typst";
        version = cargoToml.workspace.package.version;

        # Crane-based Nix flake configuration.
        # Based on https://github.com/ipetkov/crane/blob/master/examples/trunk-workspace/flake.nix
        craneLib = crane.mkLib pkgs;

        # Typst files to include in the derivation.
        # Here we include Rust files, docs and tests.
        src = lib.sourceByRegex ./. [
          "(docs|crates|tests)(/.*)?"
          ''Cargo\.(toml|lock)''
          ''build\.rs''
        ];

        # Typst derivation's args, used within crane's derivation generation
        # functions.
        commonCraneArgs = {
          inherit src pname version;

          buildInputs = [
            pkgs.openssl
          ] ++ (lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.CoreServices
            pkgs.libiconv
          ]);

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.openssl.dev
          ];
        };

        # Derivation with just the dependencies, so we don't have to keep
        # re-building them.
        cargoArtifacts = craneLib.buildDepsOnly commonCraneArgs;

        typst = craneLib.buildPackage (commonCraneArgs // {
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
          TYPST_VERSION =
            let
              rev = self.shortRev or "dirty";
              version = cargoToml.workspace.package.version;
            in
            "${version} (${rev})";

          meta.mainProgram = "typst";
        });
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

        devShells.default = craneLib.devShell {
          inputsFrom = [ typst ];
        };
      };
  };
}
