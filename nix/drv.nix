{
  buildPackages,
  crane,
  darwin,
  installShellFiles,
  lib,
  libiconv,
  pkgs,
  rust-bin,
  stdenv,
}: let
  canRunNf = stdenv.hostPlatform.emulatorAvailable buildPackages;
  nf = "${stdenv.hostPlatform.emulator buildPackages} $out/bin/nf";

  rustToolchain = rust-bin.stable.latest.default;
  craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

  fs = lib.fileset;
  src = fs.toSource {
    root = ../.;
    fileset = fs.unions [
      ../Cargo.toml
      ../Cargo.lock
      (fs.fileFilter (file:
        builtins.any file.hasExt [
          "json"
          "rs"
          "schema.graphql"
          "snap"
          "toml"
          "txt"
        ])
      ../.)
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

      nativeBuildInputs = [installShellFiles];

      preFixup = lib.optionalString canRunNf ''
        mkdir man
        ${nf} generate-manpage man
        installManPage man/*

        installShellCompletion --cmd nf \
          --bash <(${nf} generate-completions bash) \
          --fish <(${nf} generate-completions fish) \
          --zsh <(${nf} generate-completions zsh)
      '';
    });

  checks = {
    clippy = craneLib.cargoClippy (commonArgs
      // {
        inherit cargoArtifacts;

        cargoClippyExtraArgs = "--all-targets -- --deny warnings -W clippy::pedantic";
      });

    fmt = craneLib.cargoFmt commonArgs;
  };

  inherit rustToolchain;
}
