{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      ...
    }:
    let
      inherit (nixpkgs) lib;
    in
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        python = pkgs.python311.withPackages (
          p: with p; [
            scipy
          ]
        );

        rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p:
          ((p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
            extensions = [
              "clippy"
              "rustfmt"
            ];
          })
        );

        mkFileSet =
          files:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions (
              files
              ++ [
                (craneLib.fileset.commonCargoSources ./.)
                ./crates/cs2kz/migrations
                ./.sqlx
                ./.example.env
              ]
            );
          };

        fileSetForCrate =
          crate:
          mkFileSet [
            (craneLib.fileset.commonCargoSources crate)
          ];

        src = mkFileSet [ ];

        commonArgs = {
          inherit src;
          strictDeps = true;
          env = {
            PYO3_PYTHON = "${python}/bin/python";
            SQLX_OFFLINE = true;
          };
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        crateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
        };

        cs2kz-api = craneLib.buildPackage (
          crateArgs
          // {
            pname = "cs2kz-api";
            src = fileSetForCrate ./crates/cs2kz-api;
            cargoExtraArgs = "--bin=cs2kz-api";
            nativeBuildInputs = [ pkgs.makeWrapper ];
            preFixup = ''
              wrapProgram $out/bin/cs2kz-api \
                --prefix PATH : ${python}/bin
            '';
          }
        );

        generator = craneLib.buildPackage (
          crateArgs
          // {
            pname = "generator";
            src = fileSetForCrate ./crates/cs2kz-api;
            cargoExtraArgs = "--bin=generator -Ffake";
          }
        );

        openapi-schema = craneLib.buildPackage (
          crateArgs
          // {
            pname = "openapi";
            src = fileSetForCrate ./crates/cs2kz-api;
            cargoExtraArgs = "--bin=openapi";
          }
        );
      in
      {
        checks = {
          inherit cs2kz-api generator openapi-schema;

          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--no-deps --all-features --all-targets -- -Dwarnings";
            }
          );

          clippy-tests = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--no-deps --all-features --tests -- -Dwarnings";
            }
          );

          rustfmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        packages = {
          inherit cs2kz-api generator openapi-schema;

          dockerImage = pkgs.dockerTools.buildLayeredImage {
            name = cs2kz-api.pname;
            tag = cs2kz-api.version;
            config = {
              Cmd = [
                "${cs2kz-api}/bin/cs2kz-api"
                "--config"
                "/etc/cs2kz-api.toml"
                "--depot-downloader-path"
                "${pkgs.depotdownloader}/bin/DepotDownloader"
              ];
            };
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            rust-toolchain
            python
          ]
          ++ (with pkgs; [
            docker-client
            lazydocker
            mariadb
            mycli
            sqlx-cli
            tokio-console
            depotdownloader
            oha
          ]);

          KZ_API_ENVIRONMENT = "local";
          PYO3_PYTHON = "${python}/bin/python";
        };
      }
    );
}
