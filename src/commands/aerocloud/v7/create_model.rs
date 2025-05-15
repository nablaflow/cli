use crate::{
    aerocloud::{
        Client, new_idempotency_key,
        types::{
            CreateModelV7Params, CreateModelV7ParamsFilesItem, FileUnit, ModelV7,
            ModelV7FilesItem, Quaternion, UpdatePartV7Params,
        },
    },
    args::Args,
};
use color_eyre::eyre::{self, WrapErr, bail};
use reqwest::header::CONTENT_LENGTH;
use std::path::PathBuf;
use tokio::{
    fs::{self, File as AsyncFile},
    task::JoinSet,
};
use tracing::{debug, info};

#[derive(Debug, serde::Deserialize, Clone)]
struct CreateModelParams {
    name: String,
    reusable: bool,
    files: Vec<CreateModelFileParams>,

    #[serde(default)]
    all_rolling_parts: bool,

    #[serde(default)]
    rolling_parts: Vec<String>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct CreateModelFileParams {
    path: PathBuf,
    unit: FileUnit,
    rotation: Option<Quaternion>,
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
                                    "file {:?} does not have a file name",
                                    file_params.path
                                )
                            })?
                            .to_str()
                            .ok_or_else(|| {
                                eyre::eyre!(
                                    "{:?} contains invalid utf-8 chars",
                                    file_params.path
                                )
                            })?
                            .try_into()
                            .wrap_err("putting back filename into file name")?,
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

    let params = serde_json::from_str::<CreateModelParams>(params)
        .wrap_err("failed to parse json")?;

    for file in &params.files {
        if !fs::try_exists(&file.path).await? {
            bail!("file {:?} does not exist", file.path);
        }
    }

    let ModelV7 {
        id: model_id,
        files,
        ..
    } = client
        .models_v7_create(&idempotency_key, &params.clone().try_into()?)
        .await?
        .into_inner();

    debug!("model created with id {model_id}");

    upload_files(client, &files, &params).await?;

    let idempotency_key = new_idempotency_key();

    let model = client
        .models_v7_finalise(&model_id, &idempotency_key)
        .await?;

    mark_parts_as_rolling(client, &model, &params).await?;

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

async fn mark_parts_as_rolling(
    client: &Client,
    model: &ModelV7,
    params: &CreateModelParams,
) -> eyre::Result<()> {
    let mut part_to_mark_as_rolling = model
        .files
        .iter()
        .flat_map(|file| file.parts.iter())
        .collect::<Vec<_>>();

    if !params.all_rolling_parts {
        part_to_mark_as_rolling
            .retain(|part| params.rolling_parts.contains(&part.name));
    }

    for part in part_to_mark_as_rolling {
        let _ = client
            .parts_v7_update(
                &model.id,
                &part.id,
                &UpdatePartV7Params {
                    rolling: Some(true),
                },
            )
            .await?;

        info!("marked part `{}` as rolling", part.id);
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
            .unwrap();

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
        .wrap_err_with(|| format!("failed to open file {path:?}"))?;

    let metadata = body.metadata().await?;

    let res = client
        .put(upload_url)
        .body(body)
        .header(CONTENT_LENGTH, metadata.len().to_string())
        .send()
        .await
        .wrap_err_with(|| format!("failed to upload file {path:?}"))?;

    if res.status() != 200 {
        bail!("failed to upload file {path:?}: {res:?}");
    }

    info!("uploaded {:?}", path);

    Ok(())
}
