{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-24.05;
    flake-utils.url = github:numtide/flake-utils;
    rust-overlay.url = github:oxalica/rust-overlay;
    crane.url = github:ipetkov/crane;
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, crane, ... }: flake-utils.lib.eachDefaultSystem (system:
    let
      overlays = [ (import rust-overlay) ];

      pkgs = import nixpkgs {
        inherit system overlays;
      };

      rust = pkgs.callPackage ./nix/rust.nix {
        inherit crane;
      };

      cs2kz-api = pkgs.callPackage ./nix/cs2kz-api.nix {
        inherit crane;
      };
    in
    {
      packages = {
        inherit cs2kz-api;

        dockerImage = pkgs.callPackage ./nix/docker.nix {
          inherit cs2kz-api;
        };

        default = cs2kz-api;
      };

      devShells = {
        default = pkgs.callPackage ./nix/dev-shell.nix {
          inherit (rust) rust-nightly mkToolchain;
        };
      };
    });
}
