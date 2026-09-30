use crate::{
    aerocloud::types::{
        CreateModelV7Params, CreateModelV7ParamsFilesItem, FileUnit, Filename,
        Quaternion, UpdatePartV7Params,
    },
    args::Args,
    commands::aerocloud::v7::model_submission::{
        FileSpec, ModelSpec, ModelSubmitter,
    },
    progress::no_progress,
};
use bytesize::ByteSize;
use color_eyre::eyre::{self, WrapErr, bail};
use itertools::Itertools;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::fs;

#[derive(Debug, serde::Deserialize, Clone)]
struct CreateModelParams {
    name: String,
    reusable: bool,
    files: Vec<CreateModelFileParams>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct CreateModelFileParams {
    path: PathBuf,
    unit: FileUnit,
    rotation: Option<Quaternion>,
    parts: HashMap<String, UpdatePartV7Params>,
}

fn filename(path: &Path) -> eyre::Result<Filename> {
    path.file_name()
        .ok_or_else(|| {
            eyre::eyre!("file `{}` does not have a file name", path.display())
        })?
        .to_str()
        .ok_or_else(|| {
            eyre::eyre!("file `{}` contains invalid utf-8 chars", path.display())
        })?
        .try_into()
        .wrap_err_with(|| format!("file `{}` is not compatible", path.display()))
}

impl TryInto<CreateModelV7Params> for CreateModelParams {
    type Error = eyre::Error;

    fn try_into(self) -> eyre::Result<CreateModelV7Params> {
        Ok(CreateModelV7Params {
            name: self.name,
            reusable: self.reusable,
            files: self
                .files
                .into_iter()
                .map(|file_params| {
                    Ok(CreateModelV7ParamsFilesItem {
                        name: filename(&file_params.path)?,
                        unit: file_params.unit,
                        rotation: file_params
                            .rotation
                            .map_or([1.0, 0.0, 0.0, 0.0], |q| q.0),
                    })
                })
                .collect::<eyre::Result<_>>()?,
        })
    }
}

pub async fn run(
    args: &Args,
    submitter: &ModelSubmitter,
    params: &str,
) -> eyre::Result<()> {
    let params: CreateModelParams =
        serde_json::from_str(params).wrap_err("failed to parse json")?;

    let files = validate_files(&params.files).await?;

    let model_id = submitter
        .submit_model(
            ModelSpec {
                params: params.try_into()?,
                files,
            },
            no_progress(),
        )
        .await?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "model_id": model_id,
            }))?
        );
    } else {
        println!("Created model with id {model_id}");
    }

    Ok(())
}

async fn validate_files(
    files: &[CreateModelFileParams],
) -> eyre::Result<Vec<FileSpec>> {
    let mut specs = Vec::with_capacity(files.len());

    for file in files {
        let attr = fs::metadata(&file.path).await.with_context(|| {
            format!("checking file `{}`", file.path.display())
        })?;

        if !attr.is_file() {
            bail!("file {} does not exist", file.path.display());
        }

        specs.push(FileSpec {
            path: file.path.clone(),
            filename: filename(&file.path)?,
            size: ByteSize::b(attr.len()),
            parts: file
                .parts
                .iter()
                .map(|(name, params)| (name.clone(), params.clone()))
                .collect(),
        });
    }

    if !files.iter().map(|file| file.path.file_name()).all_unique() {
        bail!("all file names must be unique");
    }

    Ok(specs)
}
