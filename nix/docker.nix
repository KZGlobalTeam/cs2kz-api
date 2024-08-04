{ pkgs, cs2kz-api, ... }:

pkgs.dockerTools.buildLayeredImage {
  name = "cs2kz-api";
  tag = "latest";
  contents = [ pkgs.depotdownloader ];
  config = {
    Cmd = [ "${cs2kz-api}/bin/cs2kz-api" ];
  };
}
