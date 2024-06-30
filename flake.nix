{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-24.05;
    flake-utils.url = github:numtide/flake-utils;
    rust-overlay.url = github:oxalica/rust-overlay;
    crane.url = github:ipetkov/crane;
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, crane, ... }: flake-utils.lib.eachDefaultSystem (system:
    let
      inherit (nixpkgs) lib;
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
      };

      # fixed versions of stable & nightly toolchains
      rust-stable = pkgs.rust-bin.stable."1.79.0";
      rust-nightly = pkgs.rust-bin.nightly."2024-06-25";

      # function for instantiating a stable toolchain with a set of components
      mkToolchain = extensions: rust-stable.minimal.override {
        inherit extensions;
      };

      # tell crane which toolchain to use for builds
      craneLib = (crane.mkLib pkgs).overrideToolchain (mkToolchain [ "rust-src" ]);

      # common attributes for building both dependencies and the application itself
      commonArgs = {
        src = lib.cleanSourceWith {
          src = ./.;
          name = "source";
          filter = path: type: (craneLib.filterCargoSources path type)
            || (builtins.any (pattern: ((builtins.match pattern path) != null)) [
            ".*README.md$"
            ".*sqlx/query-.*json$"
            ".*database/fixtures/.*sql$"
            ".*database/migrations/.*sql$"
          ]);
        };

        nativeBuildInputs = [
          (mkToolchain [ "rust-src" "clippy" ])
        ];
      };

      # build dependencies & api crate
      cs2kz-api = craneLib.buildPackage (commonArgs // {
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # most of the tests are integration tests that need docker
        # they all run in CI anyway
        doCheck = false;
      });
    in
    {
      packages = {
        inherit cs2kz-api;
        default = cs2kz-api;

        dockerImage = pkgs.dockerTools.buildLayeredImage {
          name = "cs2kz-api";
          tag = "latest";
          contents = [ pkgs.depotdownloader ];
          config = {
            Cmd = [ "${cs2kz-api}/bin/cs2kz-api" ];
          };
        };
      };

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = [
          (mkToolchain [ "rust-src" "clippy" "rust-analyzer" ])
          rust-nightly.rustfmt
        ] ++ (with pkgs; [
          just
          docker-client
          mariadb_110
          sqlx-cli
          tokio-console
          depotdownloader
        ]);
      };

      apps =
        let
          mkRunnable = { script, env ? { } }: (
            let
              wrapped = pkgs.writeShellScript "runnable" ''
                ${lib.strings.concatStrings (lib.mapAttrsToList (key: value: ''export ${key}="${value}"'') env)}
                ${script}
              '';
            in
            {
              type = "app";
              program = "${wrapped}";
            }
          );
        in
        {
          rustfmt = mkRunnable {
            script = ''
              "${rust-nightly.rustfmt}/bin/cargo-fmt" --all "$@"
            '';
          };

          precommit = mkRunnable {
            script = ''
              set -eux
              just clippy
              nix run .#rustfmt
              just doc
              just sqlx-cache
            '';
          };

          console = mkRunnable {
            script = ''
              "${rust-nightly.cargo}/bin/cargo" run -F console "$@"
            '';
            env = {
              RUSTFLAGS = "--cfg tokio_unstable";
            };
          };
        };
    });
}
