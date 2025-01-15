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

> [!TIP]
> All commands accept `--json` after `nf` to output JSON instead of human readable text, so that the CLI can be comfortably used in scripts.

> [!TIP]
> Use `--help` on every subcommand to see their documentation.

### AeroCloud v7

In order to submit a simulation you need a project and a model:
- `nf aerocloud v7 list-projects` or `nf aerocloud v7 create-project "project name"`, note down the project ID.
- `nf aerocloud v7 create-model $PATH_TO_JSON`, see [examples/aerocloud/v7/create_model.json](examples/aerocloud/v7/create_model.json) as starting point.
  Note down the model id.
- `nf aerocloud v7 create-simulation --model-id $MODEL_ID --project-id $PROJECT_ID $PATH_TO_JSON`, see [examples/aerocloud/v7/create_simulation.json](examples/aerocloud/v7/create_simulation.json) as starting point.

> [!TIP]
> All commands that consume a JSON from a path also accept JSON from stdin, be sure to pass `-` instead of the file.  
> This facilitates usage from scripts.
