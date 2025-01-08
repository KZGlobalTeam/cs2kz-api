{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, utils, ... }@inputs: utils.lib.eachDefaultSystem (system:
    let
      overlays = [ (import inputs.rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
      };

      mkToolchain = components: pkgs.rust-bin.nightly."2025-01-08".minimal.override {
        extensions = [ "rust-src" ] ++ components;
      };
    in
    {
      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          docker-client
          lazydocker
          mycli
          sqlx-cli
          tokio-console
          depotdownloader
          oha

          (mkToolchain [ "clippy" "rustfmt" "rust-analyzer" ])
        ];
      };
    });
}
