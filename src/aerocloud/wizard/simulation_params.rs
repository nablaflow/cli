use crate::aerocloud::{
    extra_types::{CreateSimulationV7ParamsFromJson, FileV7ParamsFromJson},
    types::{Filename, Id},
};
use color_eyre::eyre::{self, WrapErr};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct SimulationParams {
    pub params: CreateSimulationV7ParamsFromJson,
    pub model_state: ModelState,
    pub files: Vec<FileParams>,
}

impl SimulationParams {
    pub fn many_from_root_dir(root_dir: &Path) -> eyre::Result<Vec<Self>> {
        if !root_dir.is_dir() {
            eyre::bail!("`{}` is not a directory", root_dir.display());
        }

        let mut sims_params = vec![];

        for entry in fs::read_dir(root_dir).wrap_err_with(|| {
            format!("listing root dir `{}`", root_dir.display())
        })? {
            let entry = entry.wrap_err("failed to access path while listing")?;

            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            sims_params.push(Self::from_dir(&path).wrap_err_with(|| {
                format!(
                    "failed to build simulation params from dir `{}`",
                    path.display()
                )
            })?);
        }

        Ok(sims_params)
    }

    pub fn from_dir(dir: &Path) -> eyre::Result<Self> {
        let params_path = dir.join("params.json");

        let dir_name = dir.file_name().ok_or_else(|| {
            eyre::eyre!("no file name for path `{}`", dir.display())
        })?;
        let sim_name = dir_name
            .to_str()
            .ok_or_else(|| {
                eyre::eyre!(
                    "dir name {:?} contains invalid utf-8 characters",
                    dir_name
                )
            })?
            .to_owned();

        let params = if params_path.exists() {
            let mut params: CreateSimulationV7ParamsFromJson =
                serde_json::from_slice(&fs::read(&params_path).wrap_err_with(
                    || format!("failed to read `{}`", params_path.display()),
                )?)
                .wrap_err_with(|| {
                    format!("failed to parse `{}`", params_path.display())
                })?;

            params.name = sim_name;

            params
        } else {
            CreateSimulationV7ParamsFromJson {
                name: sim_name,
                ..Default::default()
            }
        };

        let mut files = vec![];

        for entry in fs::read_dir(dir)
            .wrap_err_with(|| format!("listing dir `{}`", dir.display()))?
        {
            let entry = entry.wrap_err("failed to access path while listing")?;

            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            if let Some("json") =
                path.extension().and_then(|os_str| os_str.to_str())
            {
                continue;
            }

            let filename = path.file_name().ok_or_else(|| {
                eyre::eyre!("no file name for path `{}`", path.display())
            })?;
            let filename = filename.to_str().ok_or_else(|| {
                eyre::eyre!(
                    "file name {:?} contains invalid utf-8 characters",
                    filename
                )
            })?;

            if Filename::try_from(filename).is_err() {
                continue;
            }

            let file_params_path = path.with_extension("json");

            let file_params = if file_params_path.exists() {
                serde_json::from_slice(
                    &fs::read(&file_params_path).wrap_err_with(|| {
                        format!("failed to read `{}`", file_params_path.display())
                    })?,
                )
                .wrap_err_with(|| {
                    format!("failed to parse `{}`", file_params_path.display())
                })?
            } else {
                FileV7ParamsFromJson::default()
            };

            file_params.ensure_is_valid().wrap_err_with(|| {
                eyre::eyre!(
                    "validating file params from `{}`",
                    file_params_path.display()
                )
            })?;

            files.push(FileParams {
                path,
                params: file_params,
                state: FileState::Pending,
            });
        }

        files.sort_unstable_by(|a, b| a.path.cmp(&b.path));

        Ok(Self {
            params,
            files,
            model_state: ModelState::Pending,
        })
    }
}

#[derive(Debug)]
pub struct FileParams {
    pub path: PathBuf,
    pub params: FileV7ParamsFromJson,
    pub state: FileState,
}

#[derive(Debug)]
pub enum FileState {
    Pending,
    Uploaded { id: Id },
}

#[derive(Debug)]
pub enum ModelState {
    Pending,
    Created { id: Id },
    Finalized { id: Id },
}
