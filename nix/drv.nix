{
  pkgs,
  lib,
  stdenv,
  libiconv,
  darwin,
  rust-bin,
  crane,
}:
let
  rustToolchain = rust-bin.stable.latest.default;
  craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
  commonArgs = {
    strictDeps = true;

    src = craneLib.cleanCargoSource ../.;

    buildInputs =
      []
      ++ lib.optionals stdenv.isDarwin [
        libiconv
        darwin.apple_sdk.frameworks.Security
        darwin.apple_sdk.frameworks.SystemConfiguration
      ];

    # nativeBuildInputs = [
    #   nasm
    #   cmake
    # ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in {
  app = craneLib.buildPackage (commonArgs // {
    inherit cargoArtifacts;
  });

  checks = {
    clippy = craneLib.cargoClippy (commonArgs // {
      inherit cargoArtifacts;

      cargoClippyExtraArgs = "--all-targets -- --deny warnings";
    });

    fmt = craneLib.cargoFmt commonArgs;
  };

  inherit rustToolchain;
}
