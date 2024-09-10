{ pkgs, crane, ... }:

let
  rust-stable = pkgs.rust-bin.stable."1.81.0";
  rust-nightly = pkgs.rust-bin.nightly."2024-09-10";

  mkToolchain = extensions: rust-stable.minimal.override {
    inherit extensions;
  };

  craneLib = (crane.mkLib pkgs).overrideToolchain (mkToolchain [ "rust-src" ]);

in
{
  inherit rust-stable rust-nightly mkToolchain craneLib;
}
