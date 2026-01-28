# 1.1.0 - 2026-01-28

## AeroCloud

### Features

  - Add `boundary_layer_treatment` field to v7 simulations [#138](https://github.com/nablaflow/cli/pull/138)

### Improvements

  - Use text in place of emojis for simulation statuses.
  - Improve formatting of boundary layer treatment.


# 1.0.0 - 2026-01-19

### Features

  - Check for latest CLI version. This check can be disabled and won't be performed in any case when running with `--json`.


# 1.0.0-rc.2 - 2026-01-10

## AeroCloud

### Improvements

  - Optimise for large file uploads. [#135](https://github.com/nablaflow/cli/pull/135)


# 1.0.0-rc.1 - 2026-01-05

## AeroCloud

### Features

#### v7

  - In the interactive UI, add the ability to reload sims from disk using `r`.


# 1.0.0-beta.1 - 2025-12-30

Initial testing release with AeroCloud support.

## AeroCloud

### Features

#### v7

  - List/create/delete projects.
  - Create models for simulations.
  - List/create/delete simulations.
  - Various utils to integrate the CLI in scripting envs.
  - An interactive UI for submitting a batch of simulations.

#### v6 (end of life, read-only):

  - List/create/delete projects.
  - List/delete simulations.
