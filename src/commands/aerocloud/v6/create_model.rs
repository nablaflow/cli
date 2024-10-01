use crate::{
    args::Args,
    config::Config,
    http,
    queries::aerocloud::{
        self, CreateModelV6Mutation, CreateModelV6MutationParams,
        FileUploadStrategy, InputFileV6, InputModelV6,
    },
};
use color_eyre::eyre::{self, bail, WrapErr};
use cynic::{http::ReqwestExt, MutationBuilder};
use reqwest::header::CONTENT_LENGTH;
use serde::Deserialize;
use std::{fs, path::PathBuf, time::Duration};
use tokio::{fs::File as AsyncFile, task::JoinSet};
use tracing::{debug, error, info};

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum FileUnit {
    Mm,
    Cm,
    M,
    Inches,
}

impl From<FileUnit> for aerocloud::FileUnit {
    fn from(val: FileUnit) -> Self {
        match val {
            FileUnit::Mm => Self::Mm,
            FileUnit::Cm => Self::Cm,
            FileUnit::M => Self::M,
            FileUnit::Inches => Self::Inches,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct File {
    pub name: String,
    pub unit: FileUnit,
    pub orientation: Option<[f64; 4]>,
    pub path: PathBuf,
}

impl From<File> for InputFileV6 {
    fn from(val: File) -> Self {
        Self {
            name: val.name,
            unit: val.unit.into(),
            orientation: val.orientation,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Model {
    pub name: String,
    pub reusable: bool,
    pub files: Vec<File>,
}

impl From<Model> for InputModelV6 {
    fn from(val: Model) -> Self {
        Self {
            name: val.name,
            reusable: val.reusable,
            files: val.files.into_iter().map(Into::into).collect(),
        }
    }
}

pub async fn run(args: &Args, config: &Config, params: &str) -> eyre::Result<()> {
    let model =
        serde_json::from_str::<Model>(params).wrap_err("failed to parse json")?;

    for file in &model.files {
        if !fs::exists(&file.path)? {
            bail!("file {:?} does not exist", file.path);
        }
    }

    let op = CreateModelV6Mutation::build(CreateModelV6MutationParams {
        input: model.clone().into(),
    });

    let (client, endpoint) =
        http::build_aerocloud_client(&config.token, &config.hostname)?;

    debug!(
        "{endpoint}, query: {}, variables: {:?}",
        op.query, op.variables
    );

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to mutate")?;

    if let Some(errors) = res.errors {
        for error in errors {
            error!("{}", error);
        }

        bail!("mutation returned errors");
    }

    let model_for_upload = res
        .data
        .ok_or_else(|| eyre::eyre!("bad response"))?
        .create_model_v6;

    debug!("model created with id {}", model_for_upload.id.inner());

    let mut set = JoinSet::new();

    let s3_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .wrap_err("building http client")?;

    for file in model_for_upload.files {
        assert_eq!(FileUploadStrategy::S3, file.strategy);

        let input_file =
            model.files.iter().find(|f| f.name == file.name).unwrap();
        set.spawn(upload_file(
            file.upload_url.clone(),
            input_file.path.clone(),
            s3_client.clone(),
        ));
    }

    for res in set.join_all().await {
        let () = res.wrap_err("failed to upload file")?;
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "model_id": model_for_upload.id,
            }))?
        );
    } else {
        println!("Created model with id {}", model_for_upload.id.inner());
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
        return Err(eyre::eyre!("failed to upload file {path:?}: {:?}", res));
    }

    info!("uploaded {:?}", path);

    Ok(())
}
