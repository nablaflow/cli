# NablaFlow CLI &nbsp; [![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)

> [!TIP]
> The CLI is already in the development environment of AeroCloud starting from v7, so you don't need to install it.

Can be run directly via `nix run git+ssh://git@github.com/nablaflow/cli`.

## Installation

### As flake

TODO, see example [here](https://github.com/nablaflow/aerocloud/blob/324600cb9b4f66638a060f301df83fe05eb6743a/flake.nix#L50)

### Stand-alone (Linux, macOS, Windows)

TODO

## Usage

As NablaFlow's API requires authentication, you first need to get a personal access token.  
Once you have one, set it using `nf config set-auth-token $TOKEN`.

If you need to use the API on staging, then also `nf config set-hostname https://api.nablaflow-staging.io` is required.

The concept of profiles will be added in the future to facilitate switching between staging and production.

> [!TIP]
> Check that everything was setup correctly by running `nf aerocloud current-user`.

All commands accept `--json` after `nf` to output JSON instead of human readable text, so that the CLI can be comfortably used in scripts.

Use `--help` on every subcommand to see their documentation.

### AeroCloud v6

In order to submit a simulation you need a project and a model:
- `nf aerocloud v6 list-projects`, note down the project ID.
- `nf aerocloud v6 create-model $PATH_TO_JSON`, see [examples/aerocloud/v6/create_model.json](examples/aerocloud/v6/create_model.json) as starting point.
  Note down the model id.
- `nf aerocloud v6 create-simulation --model-id $MODEL_ID --project-id $PROJECT_ID $PATH_TO_JSON`, see [examples/aerocloud/v6/create_simulation.json](examples/aerocloud/v6/create_simulation.json) as starting point.
