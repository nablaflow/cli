{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";

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
        # inputsFrom = builtins.attrValues self.checks.${system};

        packages = with pkgs; [
          nablaflow-cli.rustToolchain
          cargo-outdated
        ];
      };

      apps.default = flake-utils.lib.mkApp {
        drv = nablaflow-cli.app;
      };

      formatter = pkgs.alejandra;
    });
}
