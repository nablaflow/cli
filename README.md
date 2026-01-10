# NablaFlow CLI &nbsp; [![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)

## Installation

### As flake

Can be run directly via `nix run github:nablaflow/cli/v1.0.0-rc.2`.

### Stand-alone (Linux, macOS, Windows)

Follow instructions in [releases](https://github.com/nablaflow/cli/releases).

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

#### Batch

`nf aerocloud v7 batch $dir` provides an interactive UI to review and submit a batch of simulations.

Inside the UI, follow the instructions and available commands at the bottom of each block.

> [!TIP]
> When submitting simulations, stuff can go wrong: network timeouts, invalid parameters and such.  
> Each succesfully submitted simulation will be marked as such and won't be resent, while allowing you to make changes to JSON files
 and reload them.

The expected structure of the passed `$dir` is:

```
dir
├── simulation-1      # `simulation-1` is going to be the name of the simulation.
│   ├── model-1.stl   # `model` can be any valid UTF-8 filename.
│   ├── model-1.json  # Provides additional params on the `.stl` above (unit, parts, ...). *File names must match!*
│   ├── model-2.obj
│   ├── model-2.json
│   ├── params.json   # Simulation params.
└── simulation-2
    ├── donut-with-parts.json
    ├── donut-with-parts.obj
    ├── params.json
```

You can find an example under [examples/aerocloud/v7/batch](examples/aerocloud/v7/batch)
