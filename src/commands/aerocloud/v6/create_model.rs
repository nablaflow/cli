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
use serde::Deserialize;
use std::{fs, path::PathBuf, time::Duration};
use tokio::{fs::File as AsyncFile, task::JoinSet};
use tracing::{debug, info};

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

#[derive(Deserialize, Debug)]
pub struct File {
    pub name: String,
    pub unit: FileUnit,
    pub orientation: Option<[f64; 4]>,
    pub path: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct Model {
    pub name: String,
    pub reusable: bool,
    pub files: Vec<File>,
}

pub async fn run(args: &Args, config: &Config, params: &str) -> eyre::Result<()> {
    let model =
        toml::from_str::<Model>(params).wrap_err("failed to parse toml")?;

    for file in &model.files {
        if !fs::exists(&file.path)? {
            bail!("file {:?} does not exist", file.path);
        }
    }

    let op = CreateModelV6Mutation::build(CreateModelV6MutationParams {
        input: InputModelV6 {
            name: model.name.clone(),
            reusable: model.reusable,
            files: model
                .files
                .iter()
                .map(|file| InputFileV6 {
                    name: file.name.clone(),
                    unit: file.unit.into(),
                    orientation: file.orientation,
                })
                .collect(),
        },
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

    client
        .post(upload_url)
        .body(body)
        .send()
        .await
        .wrap_err_with(|| format!("failed to upload file {path:?}"))?;

    info!("uploaded {:?}", path);

    Ok(())
}
