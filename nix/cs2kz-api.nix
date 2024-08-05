/**
 * CS2KZ API - the core infrastructure for CS2KZ.
 * Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see https://www.gnu.org/licenses.
 */

{ lib, pkgs, crane, ... }:

let
  inherit (pkgs.callPackage ./rust.nix { inherit crane; }) mkToolchain craneLib;

  commonArgs = {
    src = lib.cleanSourceWith {
      src = ../.;
      name = "source";
      filter = path: type: (craneLib.filterCargoSources path type)
        || (builtins.any (pattern: ((builtins.match pattern path) != null)) [

        # some modules embed READMEs as their doc comment
        ".*README.md$"

        # required by sqlx macros
        ".*sqlx/query-.*json$"
        ".*database/fixtures/.*sql$"
        ".*database/migrations/.*sql$"

        # required by problem-details docgen
        ".*static/.*(html|css)$"
      ]);
    };

    nativeBuildInputs = [
      (mkToolchain [ "rust-src" "clippy" ])
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in

craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts;

  # A lot of tests are integration tests that require docker.
  # They run in CI anyway, so it's not too important to have nix run them as well.
  doCheck = false;
})
