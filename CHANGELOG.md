# Unreleased

## AeroCloud

### Breaking changes

  - `--file-upload-concurrency` now only applies to small files (default raised from 1 to 8). Large files are controlled by the new `--large-file-upload-concurrency` (default 1), and `--large-file-threshold` (default 64MiB) decides which is which. Previously neither this flag nor `--api-request-concurrency` actually limited concurrent requests.
  - `--api-request-concurrency` now only applies to creating models and simulations (default raised from 8 to 16). Finalising models is not limited.
  - `v7 create-model` now fails when a part named in the params is not found in the uploaded file, like `v7 batch` does, instead of only logging a warning.

### Improvements

  - `v7 create-model` and `v7 batch`: small files upload in parallel, large ones sequentially. Each model is finalised as soon as its own files are uploaded, in parallel with other uploads, and its simulation is created right after.
  - Retry failed uploads and API requests with exponential backoff. Every upload attempt fetches a fresh upload url, so queued uploads no longer fail on expired urls.
  - Finalising a model has its own 15 minute HTTP timeout.
  - Read files in 1MiB chunks when uploading.
  - `v7 batch`: logs no longer get written over the UI when `--log-to-path` is not set.
  - `v7 batch`: upload progress stays accurate across retries and failed simulations, and no longer slows uploads down.

# 1.7.0 - 2026-09-23

## General

  - Sign Windows `.exe`s via Azure Artifact Signing.
  - Temporarily drop native support for Windows ARM (`aarch64-pc-windows-msvc`), as `cargo-dist` is not able to sign them. Will be put back once it does.

## AeroCloud

### Features

  - Add support for `has_tyre_plinth` to v7 simulations

# 1.6.0 - 2026-09-02

## AeroCloud

### Features

  - Add support for `symmetry` to v7 simulations [#174](https://github.com/nablaflow/cli/pull/174)

# 1.5.0 - 2026-08-07

## AeroCloud

### Breaking changes

  - Removed flags:
      - `--http-timeout-secs`, see below.

### Features

  - Add support for enabling `assistant` to v7 simulations.

### Improvements

  - In `batch`, sort simulation names lexicographically.
  - Concurrency limits on HTTP requests to avoid rate-limits.
    New flags:
      - `--api-http-timeout-secs`
      - `--file-upload-http-timeout-secs`
      - `--file-upload-concurrency`
      - `--api-request-concurrency`
    Removed flags:
      - `--http-timeout-secs`

### Chores

  - Bump rustc to 1.97.1
  - Bump flake inputs
  - Bump rust deps

# 1.4.0 - 2026-07-14

## AeroCloud

### Features

  - Add support for `flow_monitor` to v7 simulations [#167](https://github.com/nablaflow/cli/pull/167)

# 1.3.0 - 2026-06-04

## AeroCloud

### Features

  - Add support for `ceiling` to v7 simulations [#160](https://github.com/nablaflow/cli/pull/160)

# 1.2.0 - 2026-03-02

## AeroCloud

### Features

  - Add `v7 list-reusable-models` command. [#146](https://github.com/nablaflow/cli/pull/146)
  - Add support for reusable models in `v7 batch`. [#147](https://github.com/nablaflow/cli/pull/147)

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
