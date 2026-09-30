use crate::{
    aerocloud::types::{Id, SimulationV7},
    commands::aerocloud::v7::{
        batch::{
            Event,
            simulation_params::{ModelParams, SimulationParams},
        },
        model_submission::{FileSpec, ModelSpec, ModelSubmitter},
    },
    progress::{ProgressFn, ProgressScope, channel_reporter},
};
use color_eyre::eyre;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub fn submit_batch_in_background(
    project_id: &Id,
    mut sims: Vec<SimulationParams>,
    submitter: &ModelSubmitter,
    cancellation_token: &CancellationToken,
    tx: &mpsc::Sender<Event>,
) {
    // Smaller models first, so that they get finalised while larger ones are still uploading.
    sims.sort_by_key(SimulationParams::files_size);

    let progress = channel_reporter(tx.clone(), Event::UploadProgressed);

    for sim in sims {
        let project_id = project_id.clone();
        let submitter = submitter.clone();
        let cancellation_token = cancellation_token.clone();
        let progress = progress.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            let internal_id = sim.internal_id;

            // NOTE: if the simulation fails, its upload progress gets rolled back and its
            // files are removed from the total once `SimSubmitted` is received.
            let scope = ProgressScope::new(progress);

            tokio::select! {
                () = cancellation_token.cancelled() => {
                    tracing::debug!("cancellation token triggered");
                }
                res = submit_sim(project_id, sim, &submitter, scope.progress_fn()) => {
                    if res.is_ok() {
                        scope.commit();
                    } else {
                        drop(scope);
                    }

                    tx.send(Event::SimSubmitted { internal_id, res: res.map(Box::new) }).await?;
                }
            }

            Ok::<(), eyre::Report>(())
        });
    }
}

async fn submit_sim(
    project_id: Id,
    sim: SimulationParams,
    submitter: &ModelSubmitter,
    progress: ProgressFn,
) -> eyre::Result<SimulationV7> {
    let model_id = match &sim.model_params {
        ModelParams::Existing { model } => model.id.clone(),
        ModelParams::New { files } => {
            let spec = ModelSpec {
                params: sim.clone().into_api_create_model_params().ok_or_else(
                    || eyre::eyre!("simulation does not define a new model"),
                )?,
                files: files
                    .iter()
                    .map(|file| FileSpec {
                        path: file.path.clone(),
                        filename: file.filename.clone(),
                        size: file.size,
                        parts: file
                            .params
                            .parts
                            .iter()
                            .map(|(name, params)| (name.clone(), params.clone()))
                            .collect(),
                    })
                    .collect(),
            };

            submitter.submit_model(spec, progress).await?
        }
    };

    submitter
        .create_simulation(&sim.into_api_params(model_id, project_id))
        .await
}
