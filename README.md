# nf - The NablaFlow CLI &nbsp; [![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)

A command-line interface for creating simulations on AeroCloud and ArchiWind.

## Installation

### Stand-alone (Linux, macOS, Windows)

#### Linux/MacOS

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/nablaflow/cli/releases/download/v1.1.0/nf-installer.sh | sh
```

#### Windows

```bash
powershell -ExecutionPolicy Bypass -c "irm https://github.com/nablaflow/cli/releases/download/v1.1.0/nf-installer.ps1 | iex"
```

### As flake (Nix users)

Can be run directly using

```bash
cachix use nablaflow-public
nix run github:nablaflow/cli/v1.1.0
```

## Getting started

After installing the CLI, open the terminal and run the `nf -V` command to verify it's correctly installed.

In order to use the CLI, a personal access token is required. Head to your [AeroCloud API settings page](https://aerocloud.nablaflow.io/developer/api) and generate one with read and write permissions. The token should never be shared; keep it safe because it grants access to your account data.

Configure the CLI with the generated token by running:

```bash
nf aerocloud set-auth-token $TOKEN
```

### Working with projects

Commands below use `v7` to target the AeroCloud API version.

To list the existing projects:
```bash
nf aerocloud v7 list-projects
```

To create a new project:
```bash
nf aerocloud v7 create-project "project name"
```

### Creating a model

To create a model, start by defining a JSON manifest for it, see [examples/aerocloud/v7/create_model.json](examples/aerocloud/v7/create_model.json) as starting point.

Then run:
```bash
nf aerocloud v7 create-model $PATH_TO_JSON
```

### Creating a simulation

To create a simulation you need:
- a target project
- an input model

Follow the instructions in the previous steps to create both, note down project ID and model ID.

Then define a JSON manifest for it, see [examples/aerocloud/v7/create_simulation.json](examples/aerocloud/v7/create_simulation.json) as starting point.

Then run:

```bash
nf aerocloud v7 create-simulation --model-id $MODEL_ID --project-id $PROJECT_ID $PATH_TO_JSON
```

> [!TIP]
> All commands accept `--json` after `nf` to output JSON instead of human readable text, so that the CLI can be comfortably used in scripts.

> [!TIP]
> Use `--help` on every subcommand to see their documentation.

> [!TIP]
> All commands that consume a JSON from a path also accept JSON from stdin, be sure to pass `-` instead of the file.  
> This facilitates usage from scripts.

### Batch mode

The CLI can be used in batch mode to create multiple simulations at once.

First, prepare a folder with the JSON manifests for your simulation and the models to upload. Use the following structure as an example:

```text
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

An example is available under [examples/aerocloud/v7/batch](examples/aerocloud/v7/batch).

With the folder ready, run the following command to enter the Batch mode:

```bash
nf aerocloud v7 batch $directory
```

An interactive UI will be started where you can review and submit your simulations.

Follow the instructions and available commands at the bottom of the user interface.

> [!TIP]
> When submitting simulations, stuff can go wrong: network timeouts, invalid parameters and such.  
> Each successfully submitted simulation will be marked as such and won't be resent, while allowing you to make changes to JSON files and reload them.
