{
  pkgs,
  lib,
  stdenv,
  libiconv,
  darwin,
  rust-bin,
  crane,
}: let
  rustToolchain = rust-bin.stable.latest.default;
  craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

  fs = lib.fileset;
  src = fs.toSource {
    root = ../.;
    fileset = fs.unions [
      ../Cargo.toml
      ../Cargo.lock
      (fs.fileFilter (file: builtins.any file.hasExt ["rs" "toml" "snap" "schema.graphql"]) ../.)
    ];
  };

  commonArgs = {
    inherit src;

    strictDeps = true;

    buildInputs =
      []
      ++ lib.optionals stdenv.isDarwin [
        libiconv
        darwin.apple_sdk.frameworks.Security
        darwin.apple_sdk.frameworks.SystemConfiguration
      ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in {
  app = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
    });

  checks = {
    clippy = craneLib.cargoClippy (commonArgs
      // {
        inherit cargoArtifacts;

        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      });

    fmt = craneLib.cargoFmt commonArgs;
  };

  inherit rustToolchain;
}
