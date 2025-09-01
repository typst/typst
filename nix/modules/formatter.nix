{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule
    inputs.git-hooks.flakeModule
  ];

  perSystem = {
    treefmt = {
      projectRootFile = "flake.nix";
      programs = {
        deadnix.enable = true;
        nixfmt.enable = true;
        prettier.enable = true;
      };
      settings = {
        on-unmatched = "fatal";
        global.excludes = [
          ".cargo/*"
          ".github/*"
          "crates/*"
          "docs/*"
          "tests/*"
          "tools/*"
          ".editorconfig"
          ".envrc"
          ".gitignore"
          "Cargo.lock"
          "Cargo.toml"
          "CITATION.cff"
          "CONTRIBUTING.md"
          "Dockerfile"
          "LICENSE"
          "NOTICE"
          "rustfmt.toml"
          "README.md"
        ];
      };
    };
  };
}
