{
  inputs,
  ...
}:
{
  imports = [
    inputs.pkgs-by-name-for-flake-parts.flakeModule
    inputs.flake-parts.flakeModules.easyOverlay
  ];

  perSystem =
    { system, config, ... }:
    {
      # Attach the `local` overlay to `pkgs`.
      # The `local` overlay exposes the packages defined in `config.packages`.
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [
          (_final: _prev: {
            local = config.packages;
          })
        ];
      };

      # Directory where the `pkgs` are defined.
      pkgsDirectory = ../pkgs;

      # Default package to be used when running `nix run`.
      packages.default = config.packages.typst;

      # Expose all packages defined in `config.packages` as overlays.
      overlayAttrs = config.packages;
    };
}
