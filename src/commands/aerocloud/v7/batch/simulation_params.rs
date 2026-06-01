use crate::aerocloud::{
    Client,
    extra_types::{CreateSimulationV7ParamsFromJson, FileV7ParamsFromJson},
    types::{
        CreateModelV7Params, CreateModelV7ParamsFilesItem,
        CreateSimulationV7Params, Filename, Id, ModelV7, Url,
    },
};
use bytesize::ByteSize;
use color_eyre::eyre::{self, WrapErr};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub enum SubmissionState {
    #[default]
    Ready,
    Sending,
    Error(String),
    Sent {
        id: Id,
        browser_url: Url,
    },
}

impl SubmissionState {
    const FILENAME: &str = "submission_state.json";

    pub async fn from_dir_or_default(dir: &Path) -> Self {
        if let Ok(buf) = fs::read(dir.join(Self::FILENAME)).await
            && let Ok(submission_state) = serde_json::from_slice(&buf)
        {
            submission_state
        } else {
            Self::default()
        }
    }

    pub async fn write(&self, dir: &Path) -> eyre::Result<()> {
        fs::write(dir.join(Self::FILENAME), &serde_json::to_vec(self)?).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ModelParams {
    New { files: Vec<FileParams> },
    Existing { model: ModelV7 },
}

impl ModelParams {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::New { files } => files.is_empty(),
            Self::Existing { model } => model.files.is_empty(),
        }
    }

    pub fn is_submittable(&self) -> bool {
        match self {
            Self::New { files } => !files.is_empty(),
            Self::Existing { .. } => true,
        }
    }

    async fn from_dir(dir: &Path) -> eyre::Result<Self> {
        let mut files = vec![];

        let mut dir_stream = fs::read_dir(dir)
            .await
            .wrap_err_with(|| format!("listing dir `{}`", dir.display()))?;

        while let Some(entry) = dir_stream
            .next_entry()
            .await
            .wrap_err("iterating dir stream")?
        {
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

            let Ok(filename) = Filename::try_from(filename) else {
                continue;
            };

            let file_params_path = path.with_extension("json");

            let file_params = if file_params_path.exists() {
                serde_json::from_slice(
                    &fs::read(&file_params_path).await.wrap_err_with(|| {
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

            let size = fs::metadata(&path)
                .await
                .map(|metadata| ByteSize::b(metadata.len()))
                .wrap_err_with(|| {
                    eyre::eyre!("reading file size of `{}`", path.display())
                })?;

            files.push(FileParams {
                path,
                filename,
                size,
                params: file_params,
            });
        }

        files.sort_unstable_by(|a, b| a.path.cmp(&b.path));

        Ok(Self::New { files })
    }

    async fn from_existing(client: &Client, id: &Id) -> eyre::Result<Self> {
        let model = client
            .models_v7_get(id)
            .await
            .wrap_err_with(|| format!("fetching reusable model {id}"))?
            .into_inner();

        Ok(Self::Existing { model })
    }
}

#[derive(Debug, Clone)]
pub struct SimulationParams {
    pub internal_id: Uuid,
    pub dir: PathBuf,
    pub params: CreateSimulationV7ParamsFromJson,
    pub model_params: ModelParams,

    pub selected: bool,
    pub submission_state: SubmissionState,
}

impl SimulationParams {
    pub async fn many_from_root_dir(
        client: &Client,
        root_dir: &Path,
    ) -> eyre::Result<Vec<Self>> {
        if !fs::metadata(root_dir).await?.is_dir() {
            eyre::bail!("`{}` is not a directory", root_dir.display());
        }

        let mut sims_params = vec![];

        let mut dir_stream =
            fs::read_dir(root_dir).await.wrap_err_with(|| {
                eyre::eyre!("error listing root dir `{}`", root_dir.display())
            })?;

        while let Some(entry) = dir_stream
            .next_entry()
            .await
            .wrap_err("iterating root dir dir stream")?
        {
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            sims_params.push(Self::from_dir(client, &path).await.wrap_err_with(
                || {
                    format!(
                        "failed to build simulation params from dir `{}`",
                        path.display()
                    )
                },
            )?);
        }

        Ok(sims_params)
    }

    #[allow(clippy::too_many_lines)]
    pub async fn from_dir(client: &Client, dir: &Path) -> eyre::Result<Self> {
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
            let buf = fs::read(&params_path).await.wrap_err_with(|| {
                format!("failed to read `{}`", params_path.display())
            })?;

            let mut params: CreateSimulationV7ParamsFromJson =
                serde_json::from_slice(&buf).wrap_err_with(|| {
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

        let model_params = if let Some(model_id) = &params.model_id {
            ModelParams::from_existing(client, model_id).await?
        } else {
            ModelParams::from_dir(dir).await?
        };

        let submission_state = SubmissionState::from_dir_or_default(dir).await;

        Ok(Self {
            internal_id: Uuid::new_v4(),
            dir: dir.into(),
            params,
            model_params,
            selected: true,
            submission_state,
        })
    }

    pub fn files_size(&self) -> ByteSize {
        let ModelParams::New { files } = &self.model_params else {
            return ByteSize::default();
        };

        files
            .iter()
            .fold(ByteSize::default(), |acc, file| acc + file.size)
    }

    pub async fn reset_submission_state(&mut self) -> eyre::Result<()> {
        self.update_submission_state(SubmissionState::default())
            .await
    }

    pub async fn update_submission_state(
        &mut self,
        state: SubmissionState,
    ) -> eyre::Result<()> {
        self.submission_state = state;
        self.submission_state.write(&self.dir).await?;
        Ok(())
    }

    pub fn is_submittable(&self) -> bool {
        self.selected
            && self.model_params.is_submittable()
            && matches!(
                self.submission_state,
                SubmissionState::Ready | SubmissionState::Error(..)
            )
    }

    pub fn into_api_create_model_params(self) -> Option<CreateModelV7Params> {
        let ModelParams::New { files } = self.model_params else {
            return None;
        };

        Some(CreateModelV7Params {
            name: self.params.name.clone(),
            reusable: false,
            files: files
                .into_iter()
                .map(|file| CreateModelV7ParamsFilesItem {
                    name: file.filename,
                    rotation: file
                        .params
                        .rotation
                        .map(|q| q.0)
                        .unwrap_or([1.0, 0.0, 0.0, 0.0]),
                    unit: file.params.unit,
                })
                .collect(),
        })
    }

    pub fn into_api_params(
        self,
        model_id: Id,
        project_id: Id,
    ) -> CreateSimulationV7Params {
        let CreateSimulationV7ParamsFromJson {
            boundary_layer_treatment,
            fluid,
            fluid_speed,
            ground_offset,
            has_ground,
            is_ground_moving,
            name,
            quality,
            revision,
            yaw_angles,
            ceiling,
            ..
        } = self.params;

        CreateSimulationV7Params {
            boundary_layer_treatment,
            fluid,
            fluid_speed,
            ground_offset,
            has_ground,
            is_ground_moving,
            model_id,
            name,
            project_id,
            quality,
            revision,
            yaw_angles,
            ceiling,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileParams {
    pub path: PathBuf,
    pub filename: Filename,
    pub size: ByteSize,
    pub params: FileV7ParamsFromJson,
}
