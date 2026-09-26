{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      nixpkgs-unstable,
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
          overlays = [
            (import rust-overlay)
            (self: super: {
              inherit (nixpkgs-unstable.legacyPackages.${super.stdenv.hostPlatform.system})
                depotdownloader
                ;

              geolite2-city =
                let
                  filename = "GeoLite2-City.mmdb";
                  version = "1.0.102";
                in
                super.stdenv.mkDerivation {
                  pname = "geolite2-city";
                  inherit version;
                  src = super.fetchurl {
                    url = "https://cdn.jsdelivr.net/npm/geolite2-city@${version}/${filename}.gz";
                    hash = "sha256-IRa/wcXhrPVu6gkz/OQWwdghyu0QfMGoP1/roAuQ24Q=";
                  };

                  buildCommand = ''
                    mkdir -p $out/share
                    cp $src $out/share/${filename}.gz
                    gzip -d $out/share/${filename}.gz
                  '';

                  nativeBuildInputs = [ super.gzip ];
                };
            })
          ];
        };

        python = pkgs.python311.withPackages (
          p: with p; [
            mariadb
            numpy
            packaging
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
                --prefix PATH : ${python}/bin \
                --prefix PATH : ${pkgs.geoipWithDatabase}/bin \
                --set GEOLITE_CITY_MMDB "${pkgs.geolite2-city}/share/GeoLite2-City.mmdb"
            '';
          }
        );

        generator = craneLib.buildPackage (
          crateArgs
          // {
            pname = "generator";
            src = fileSetForCrate ./crates/cs2kz-api;
            cargoExtraArgs = "--bin=generator -Ffake";
            nativeBuildInputs = [ pkgs.makeWrapper ];
            preFixup = ''
              wrapProgram $out/bin/generator \
                --prefix PATH : ${python}/bin
            '';
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
          inherit
            cs2kz-api
            generator
            openapi-schema
            python
            ;

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
            depotdownloader
            docker-client
            lazydocker
            mariadb
            mycli
            oha
            sqlx-cli
            tokio-console
          ]);

          KZ_API_ENVIRONMENT = "local";
          GEOLITE_CITY_MMDB = "${pkgs.geolite2-city}/share/GeoLite2-City.mmdb";
        };
      }
    );
}
