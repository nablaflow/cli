//! Creates models (uploading their files) and simulations.
//!
//! Each model goes through: create -> upload files -> finalise -> update parts.
//!
//! Uploads go through [`Uploader`]'s lanes, every attempt fetches a fresh upload url.
//!
//! Each model runs independently, so finalising (slow, but highly concurrent on the backend)
//! overlaps with uploads of other models.

use crate::{
    aerocloud::{
        Client, ResponseValue, new_idempotency_key, retry_failure,
        types::{
            CreateModelV7Params, CreateSimulationV7Params, Filename, Id, ModelV7,
            ModelV7FilesItem, SimulationV7, UpdatePartV7Params,
        },
    },
    args::Args,
    config::Config,
    http,
    progress::ProgressFn,
    retry::{Failure, Policy, retry},
    upload::Uploader,
};
use bytesize::ByteSize;
use color_eyre::eyre::{self, WrapErr};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task::JoinSet};

const API_RETRY_POLICY: Policy = Policy {
    max_attempts: 5,
    in_progress_budget: Duration::from_mins(10),
};

// Finalising can take up to 10 minutes: if an attempt times out client-side, the server
// answers 409 until it is done, then replays the recorded response.
const FINALISE_RETRY_POLICY: Policy = Policy {
    max_attempts: 5,
    in_progress_budget: Duration::from_mins(20),
};

#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub params: CreateModelV7Params,
    pub files: Vec<FileSpec>,
}

#[derive(Debug, Clone)]
pub struct FileSpec {
    pub path: PathBuf,
    pub filename: Filename,
    pub size: ByteSize,
    pub parts: Vec<(String, UpdatePartV7Params)>,
}

#[derive(Debug, Clone)]
pub struct ModelSubmitter {
    api: Client,
    finalise_api: Client,
    uploader: Uploader,
    api_permits: Arc<Semaphore>,
}

impl ModelSubmitter {
    pub fn new(args: &Args, config: &Config) -> eyre::Result<Self> {
        Ok(Self {
            api: http::build_aerocloud_client(config, args)
                .wrap_err("building api client")?,
            finalise_api: http::build_aerocloud_finalise_client(config, args)
                .wrap_err("building api client for finalising")?,
            uploader: Uploader::new(
                http::build_file_upload_client(args)
                    .wrap_err("building file upload client")?,
                args.file_upload_concurrency,
                args.large_file_upload_concurrency,
                args.large_file_threshold,
            ),
            api_permits: Arc::new(Semaphore::new(
                args.api_request_concurrency.get(),
            )),
        })
    }

    /// Creates the model, uploads its files, finalises it and updates its parts.
    pub async fn submit_model(
        &self,
        spec: ModelSpec,
        progress: ProgressFn,
    ) -> eyre::Result<Id> {
        let idempotency_key = new_idempotency_key();

        let ModelV7 { id: model_id, .. } =
            retry("creating model", API_RETRY_POLICY, || async {
                let _permit = self.api_permits.acquire().await?;

                self.api
                    .models_v7_create(&idempotency_key, &spec.params)
                    .await
                    .map(ResponseValue::into_inner)
                    .map_err(retry_failure)
            })
            .await?;

        tracing::debug!("model created with id {model_id}");

        self.upload_files(&model_id, &spec.files, &progress)
            .await
            .wrap_err("uploading files")?;

        let idempotency_key = new_idempotency_key();

        let ModelV7 { files, .. } =
            retry("finalising model", FINALISE_RETRY_POLICY, || async {
                // NOTE: not bound by `api_permits`, it would hold them for minutes, stalling
                // other requests.
                self.finalise_api
                    .models_v7_finalise(&model_id, &idempotency_key)
                    .await
                    .map(ResponseValue::into_inner)
                    .map_err(retry_failure)
            })
            .await?;

        tracing::debug!("model {model_id} finalised");

        self.update_parts(&model_id, &files, &spec.files)
            .await
            .wrap_err("updating parts")?;

        Ok(model_id)
    }

    pub async fn create_simulation(
        &self,
        params: &CreateSimulationV7Params,
    ) -> eyre::Result<SimulationV7> {
        let idempotency_key = new_idempotency_key();

        retry("creating simulation", API_RETRY_POLICY, || async {
            let _permit = self.api_permits.acquire().await?;

            self.api
                .simulations_v7_create(&idempotency_key, params)
                .await
                .map(ResponseValue::into_inner)
                .map_err(retry_failure)
        })
        .await
    }

    async fn upload_files(
        &self,
        model_id: &Id,
        files: &[FileSpec],
        progress: &ProgressFn,
    ) -> eyre::Result<()> {
        let mut set = JoinSet::new();

        for file in files {
            let this = self.clone();
            let model_id = model_id.clone();
            let file = file.clone();
            let progress = progress.clone();

            set.spawn(async move {
                this.uploader
                    .upload(
                        &file.path,
                        file.size,
                        || this.fetch_upload_url(&model_id, &file.filename),
                        &progress,
                    )
                    .await
                    .wrap_err_with(|| {
                        format!("uploading `{}`", file.path.display())
                    })
            });
        }

        // NOTE: returning early drops the set, aborting uploads still in progress.
        while let Some(res) = set.join_next().await {
            res.wrap_err("upload task failed")??;
        }

        Ok(())
    }

    async fn fetch_upload_url(
        &self,
        model_id: &Id,
        filename: &Filename,
    ) -> Result<String, Failure> {
        let model = {
            let _permit = self.api_permits.acquire().await?;

            self.api
                .models_v7_get(model_id)
                .await
                .map_err(retry_failure)?
                .into_inner()
        };

        let file = model
            .files
            .into_iter()
            .find(|f| f.name == *filename)
            .ok_or_else(|| {
                eyre::eyre!(
                    "file `{}` was not returned from the server",
                    filename.as_str()
                )
            })?;

        let upload_url = file.upload_url.ok_or_else(|| {
            eyre::eyre!("no upload url returned for file `{}`", filename.as_str())
        })?;

        Ok(upload_url.0)
    }

    async fn update_parts(
        &self,
        model_id: &Id,
        returned_files: &[ModelV7FilesItem],
        files: &[FileSpec],
    ) -> eyre::Result<()> {
        let mut set = JoinSet::new();

        for file in files {
            let returned_file = returned_files
                .iter()
                .find(|f| f.name == file.filename)
                .ok_or_else(|| {
                    eyre::eyre!(
                        "file `{}` was not returned from the server",
                        file.filename.as_str()
                    )
                })?;

            for (part_name, part_params) in &file.parts {
                let Some(part) = returned_file
                    .parts
                    .iter()
                    .find(|part| part.name == *part_name)
                else {
                    eyre::bail!(
                        "part named `{part_name}` was not found in uploaded file `{}`",
                        file.path.display()
                    );
                };

                let this = self.clone();
                let model_id = model_id.clone();
                let part_id = part.id.clone();
                let part_params = part_params.clone();

                set.spawn(async move {
                    retry("updating part", API_RETRY_POLICY, || async {
                        let _permit = this.api_permits.acquire().await?;

                        this.api
                            .parts_v7_update(&model_id, &part_id, &part_params)
                            .await
                            .map_err(retry_failure)
                    })
                    .await?;

                    tracing::info!(
                        "updated part `{part_id}` with {part_params:?}"
                    );

                    Ok::<_, eyre::Report>(())
                });
            }
        }

        while let Some(res) = set.join_next().await {
            res.wrap_err("update part task failed")?
                .wrap_err("failed to update part")?;
        }

        Ok(())
    }
}
