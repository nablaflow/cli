use crate::{
    aerocloud::{
        Client, fmt_progenitor_err, new_idempotency_key,
        types::{Id, ModelV7, ModelV7FilesItem, SimulationV7},
    },
    commands::aerocloud::v7::batch::{
        Event,
        simulation_params::{FileParams, SimulationParams},
    },
};
use bytesize::ByteSize;
use color_eyre::eyre::{self, WrapErr};
use futures_util::StreamExt;
use reqwest::header::CONTENT_LENGTH;
use std::path::PathBuf;
use tokio::{fs::File as AsyncFile, sync::mpsc, task::JoinSet};
use tokio_util::{io::ReaderStream, sync::CancellationToken};

pub fn submit_batch_in_background(
    project_id: &Id,
    sims: Vec<SimulationParams>,
    client: &Client,
    cancellation_token: &CancellationToken,
    tx: &mpsc::Sender<Event>,
) {
    for sim in sims {
        let project_id = project_id.clone();
        let client = client.clone();
        let cancellation_token = cancellation_token.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            let internal_id = sim.internal_id;

            tokio::select! {
                () = cancellation_token.cancelled() => {
                    tracing::debug!("cancellation token triggered");
                }
                res = submit_sim(project_id, sim, client, tx.clone()) => {
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
    client: Client,
    tx: mpsc::Sender<Event>,
) -> eyre::Result<SimulationV7> {
    let ModelV7 {
        id: model_id,
        files,
        ..
    } = client
        .models_v7_create(
            &new_idempotency_key(),
            &sim.clone().into_api_model_params(),
        )
        .await
        .map_err(fmt_progenitor_err)?
        .into_inner();

    upload_files(&client, &files, &sim.files, &tx)
        .await
        .wrap_err("uploading files")?;

    let ModelV7 { files, .. } = client
        .models_v7_finalise(&model_id, &new_idempotency_key())
        .await
        .map_err(fmt_progenitor_err)?
        .into_inner();

    update_parts(&client, &model_id, &files, &sim.files)
        .await
        .wrap_err("updating parts")?;

    let sim = client
        .simulations_v7_create(
            &new_idempotency_key(),
            &sim.into_api_params(model_id, project_id),
        )
        .await
        .map_err(fmt_progenitor_err)?
        .into_inner();

    Ok(sim)
}

async fn upload_files(
    client: &Client,
    files: &[ModelV7FilesItem],
    params: &[FileParams],
    tx: &mpsc::Sender<Event>,
) -> eyre::Result<()> {
    let mut set = JoinSet::new();

    for file_params in params {
        let Some(returned_file) =
            files.iter().find(|f| f.name == file_params.filename)
        else {
            eyre::bail!("file in given params was not returned from the server");
        };

        let Some(ref upload_url) = returned_file.upload_url else {
            eyre::bail!("no upload url found in response");
        };

        set.spawn(upload_file(
            file_params.size,
            upload_url.0.clone(),
            file_params.path.clone(),
            client.client.clone(),
            tx.clone(),
        ));
    }

    for res in set.join_all().await {
        let () = res.wrap_err("failed to upload file")?;
    }

    Ok(())
}

async fn upload_file(
    size: ByteSize,
    upload_url: String,
    path: PathBuf,
    client: reqwest::Client,
    tx: mpsc::Sender<Event>,
) -> eyre::Result<()> {
    let file = AsyncFile::open(&path)
        .await
        .wrap_err_with(|| format!("opening `{}`", path.display()))?;

    let mut reader_stream = ReaderStream::new(file);

    let async_stream = async_stream::stream! {
        while let Some(chunk) = reader_stream.next().await {
            if let Ok(chunk) = &chunk {
                let _ = tx.send(Event::FileUploaded(ByteSize::b(chunk.len() as u64))).await;
            }

            yield chunk;
        }
    };

    let res = client
        .put(upload_url)
        .body(reqwest::Body::wrap_stream(async_stream))
        .header(CONTENT_LENGTH, size.0.to_string())
        .send()
        .await
        .wrap_err_with(|| format!("uploading `{}`", path.display()))?;

    if res.status() != 200 {
        eyre::bail!("failed to upload `{}`: {res:?}", path.display());
    }

    tracing::debug!("uploaded {}", path.display());

    Ok(())
}

async fn update_parts(
    client: &Client,
    model_id: &Id,
    files: &[ModelV7FilesItem],
    params: &[FileParams],
) -> eyre::Result<()> {
    let mut set = JoinSet::new();

    for file_params in params {
        let Some(returned_file) =
            files.iter().find(|f| f.name == file_params.filename)
        else {
            eyre::bail!("file in given params was not returned from the server");
        };

        for (part_name, part_params) in &file_params.params.parts {
            let Some(part_id) = returned_file
                .parts
                .iter()
                .find(|part| part.name == *part_name)
                .map(|part| &part.id)
            else {
                eyre::bail!(
                    "part named `{part_name}` was not found in uploaded file `{}`",
                    file_params.path.display()
                );
            };

            let client = client.clone();
            let model_id = model_id.clone();
            let part_params = part_params.clone();
            let part_id = part_id.clone();

            let path = file_params.path.clone();

            set.spawn(async move {
                client
                    .parts_v7_update(&model_id, &part_id, &part_params)
                    .await
                    .map_err(fmt_progenitor_err)
                    .map(|_| {
                        tracing::debug!(
                            "updated part `{part_id}` for file `{}` with {part_params:?}", path.display()
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
