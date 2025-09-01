{
  lib,
  dockerTools,
  local,
  ...
}:
dockerTools.buildImage {
  name = "typst";
  tag = "latest";

  copyToRoot = [
    local.typst
  ];

  config = {
    Cmd = [ "${lib.getExe local.typst}" ];
  };
}
