use crate::{
    aerocloud::{
        Client, fmt_progenitor_err, new_idempotency_key,
        types::{
            CreateModelV7Params, CreateModelV7ParamsFilesItem, FileUnit, Id,
            ModelV7, ModelV7FilesItem, Quaternion, UpdatePartV7Params,
        },
    },
    args::Args,
    http::UPLOAD_REQ_TIMEOUT,
};
use color_eyre::eyre::{self, WrapErr, bail};
use itertools::Itertools;
use progenitor_client::ClientInfo;
use reqwest::header::CONTENT_LENGTH;
use std::{collections::HashMap, path::PathBuf};
use tokio::{
    fs::{self, File as AsyncFile},
    task::JoinSet,
};
use tracing::{debug, info, warn};

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
                        name: file_params
                            .path
                            .file_name()
                            .ok_or_else(|| {
                                eyre::eyre!(
                                    "file `{}` does not have a file name",
                                    file_params.path.display()
                                )
                            })?
                            .to_str()
                            .ok_or_else(|| {
                                eyre::eyre!(
                                    "file `{}` contains invalid utf-8 chars",
                                    file_params.path.display()
                                )
                            })?
                            .try_into()
                            .wrap_err_with(|| {
                                format!(
                                    "file `{}` is not compatible",
                                    file_params.path.display()
                                )
                            })?,
                        unit: file_params.unit,
                        rotation: file_params.rotation,
                    })
                })
                .collect::<eyre::Result<_>>()?,
        })
    }
}

pub async fn run(args: &Args, client: &Client, params: &str) -> eyre::Result<()> {
    let idempotency_key = new_idempotency_key();

    let params: CreateModelParams =
        serde_json::from_str(params).wrap_err("failed to parse json")?;

    validate_files(&params.files).await?;

    let ModelV7 {
        id: model_id,
        files,
        ..
    } = client
        .models_v7_create(&idempotency_key, &params.clone().try_into()?)
        .await
        .map_err(fmt_progenitor_err)?
        .into_inner();

    debug!("model created with id {model_id}");

    upload_files(client, &files, &params)
        .await
        .wrap_err("uploading files")?;

    let idempotency_key = new_idempotency_key();

    let ModelV7 {
        id: model_id,
        files,
        ..
    } = client
        .models_v7_finalise(&model_id, &idempotency_key)
        .await
        .map_err(fmt_progenitor_err)?
        .into_inner();

    update_parts(client, &model_id, &files, &params)
        .await
        .wrap_err("updating parts")?;

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

async fn update_parts(
    client: &Client,
    model_id: &Id,
    files: &[ModelV7FilesItem],
    params: &CreateModelParams,
) -> eyre::Result<()> {
    let mut set = JoinSet::new();

    for file in &params.files {
        let returned_file = files
            .iter()
            .find(|f| {
                Some(f.name.as_ref())
                    == file.path.file_name().and_then(|s| s.to_str())
            })
            .ok_or_else(|| {
                eyre::eyre!(
                    "file in given params was not returned from the server"
                )
            })?;

        for (part_name, part_params) in &file.parts {
            let Some(part_id) = returned_file
                .parts
                .iter()
                .find(|part| part.name == *part_name)
                .map(|part| &part.id)
            else {
                warn!(
                    "part named `{part_name}` was not found in uploaded file `{:?}`",
                    returned_file.name
                );
                continue;
            };

            let client = client.clone();
            let model_id = model_id.clone();
            let part_params = part_params.clone();
            let part_id = part_id.clone();

            set.spawn(async move {
                client
                    .parts_v7_update(&model_id, &part_id, &part_params)
                    .await
                    .map_err(fmt_progenitor_err)
                    .map(|_| {
                        info!(
                            "updated part `{}` with {:?}",
                            part_id, part_params
                        );
                    })
            });
        }
    }

    for res in set.join_all().await {
        let () = res.wrap_err("failed to update part")?;
    }

    Ok(())
}

async fn upload_files(
    client: &Client,
    files: &[ModelV7FilesItem],
    params: &CreateModelParams,
) -> eyre::Result<()> {
    let mut set = JoinSet::new();

    for file in &params.files {
        let returned_file = files
            .iter()
            .find(|f| {
                Some(f.name.as_ref())
                    == file.path.file_name().and_then(|s| s.to_str())
            })
            .ok_or_else(|| {
                eyre::eyre!(
                    "file in given params was not returned from the server"
                )
            })?;

        let upload_url = returned_file
            .upload_url
            .clone()
            .ok_or_else(|| eyre::eyre!("no upload url found in response"))?;

        set.spawn(upload_file(
            upload_url.into(),
            file.path.clone(),
            client.client().clone(),
        ));
    }

    for res in set.join_all().await {
        let () = res.wrap_err("failed to upload file")?;
    }

    Ok(())
}

async fn upload_file(
    upload_url: String,
    path: PathBuf,
    client: reqwest::Client,
) -> eyre::Result<()> {
    let body = AsyncFile::open(&path)
        .await
        .wrap_err_with(|| format!("failed to open file {}", path.display()))?;

    let metadata = body.metadata().await?;

    let res = client
        .put(upload_url)
        .body(body)
        .header(CONTENT_LENGTH, metadata.len().to_string())
        .timeout(UPLOAD_REQ_TIMEOUT)
        .send()
        .await
        .wrap_err_with(|| format!("failed to upload file {}", path.display()))?;

    if res.status() != 200 {
        bail!("failed to upload file {}: {res:?}", path.display());
    }

    info!("uploaded {}", path.display());

    Ok(())
}

async fn validate_files(files: &[CreateModelFileParams]) -> eyre::Result<()> {
    for file in files {
        let attr = fs::metadata(&file.path).await.with_context(|| {
            format!("checking file `{}`", file.path.display())
        })?;

        if !attr.is_file() {
            bail!("file {} does not exist", file.path.display());
        }
    }

    if !files.iter().map(|file| file.path.file_name()).all_unique() {
        bail!("all file names must be unique");
    }

    Ok(())
}
