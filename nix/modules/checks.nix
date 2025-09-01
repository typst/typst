{
  perSystem =
    {
      config,
      ...
    }:
    {
      checks = config.packages.typst.checks;
    };
}
