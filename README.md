# NablaFlow CLI &nbsp; [![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)

## Installation

### As flake

Can be run directly via `nix run https://github.com/nablaflow/cli`.

### Stand-alone (Linux, macOS, Windows)

TODO

## Usage

> [!TIP]
> All commands accept `--json` after `nf` to output JSON instead of human readable text, so that the CLI can be comfortably used in scripts.

> [!TIP]
> Use `--help` on every subcommand to see their documentation.

> [!TIP]
> All commands that consume a JSON from a path also accept JSON from stdin, be sure to pass `-` instead of the file.  
> This facilitates usage from scripts.

## Aerocloud

As NablaFlow's API requires authentication, you first need to [get a personal access token](https://aerocloud.nablaflow.io/developer/api).  
Once you have one, set it using `nf aerocloud set-auth-token $TOKEN`.

### AeroCloud v7

In order to submit a simulation you need a project and a model:
- `nf aerocloud v7 list-projects` or `nf aerocloud v7 create-project "project name"`, note down the project ID.
- `nf aerocloud v7 create-model $PATH_TO_JSON`, see [examples/aerocloud/v7/create_model.json](examples/aerocloud/v7/create_model.json) as starting point.
  Note down the model id.
- `nf aerocloud v7 create-simulation --model-id $MODEL_ID --project-id $PROJECT_ID $PATH_TO_JSON`, see [examples/aerocloud/v7/create_simulation.json](examples/aerocloud/v7/create_simulation.json) as starting point.
