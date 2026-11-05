{
  inputs = {
    systems.url = "github:nix-systems/triplet";

    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };

    crane.url = "github:ipetkov/crane/v0.24.0";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;

        overlays = [(import rust-overlay)];
      };

      nablaflow-cli = pkgs.callPackage ./nix/drv.nix {inherit crane;};
    in {
      checks = nablaflow-cli.checks;

      packages.default = nablaflow-cli.app;

      devShells.default = pkgs.mkShell {
        inputsFrom = builtins.attrValues self.checks.${system};

        packages = with pkgs; [
          cargo-audit
          cargo-dist
          cargo-outdated
          libwebp
          nablaflow-cli.rustToolchain
          python3
          shellcheck
          vhs
        ];

        # Font used by scripts/batch-screenshots.sh, without installing it system-wide.
        BATCH_SCREENSHOTS_FONTCONFIG = pkgs.makeFontsConf {
          fontDirectories = [pkgs.jetbrains-mono];
        };
      };

      apps.default = flake-utils.lib.mkApp {
        drv = nablaflow-cli.app;
      };

      formatter = pkgs.alejandra;
    });
}
