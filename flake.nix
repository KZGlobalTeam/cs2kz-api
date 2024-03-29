{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;
    utils.url = github:numtide/flake-utils;
  };

  outputs = { nixpkgs, utils, ... }: utils.lib.eachDefaultSystem(system: let
    pkgs = import nixpkgs {
      inherit system;
    };
  in {
    devShell = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [ rustup python3 python3Packages.scipy ];
      buildInputs = with pkgs; [
        mariadb_110
        just
        sqlx-cli
        tokio-console
      ];

      shellHook = ''
        rustup toolchain install stable
        rustup toolchain install --profile minimal nightly
        rustup override set stable
        rustup +stable component add rust-analyzer
        rustup +nightly component add rustfmt
        rustup update
      '';
    };
  });
}
