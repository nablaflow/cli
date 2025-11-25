use crate::{
    aerocloud::{
        Client, fmt_progenitor_err, new_idempotency_key,
        types::{CreateSimulationV7Params, Id},
    },
    args::Args,
    fmt::link,
};
use color_eyre::eyre::{self, WrapErr};

pub async fn run(
    args: &Args,
    client: &Client,
    model_id: Option<Id>,
    project_id: Option<Id>,
    params: &str,
) -> eyre::Result<()> {
    let idempotency_key = new_idempotency_key();

    let mut params = serde_json::from_str::<CreateSimulationV7Params>(params)
        .wrap_err("failed to parse json")?;

    if let Some(id) = model_id {
        params.model_id = id;
    }
    if let Some(id) = project_id {
        params.project_id = id;
    }

    let sim = client
        .simulations_v7_create(&idempotency_key, &params)
        .await
        .map_err(fmt_progenitor_err)?
        .into_inner();

    if args.json {
        println!("{}", serde_json::to_string(&sim)?);
    } else {
        println!(
            "Created simulation `{}` with id {} {}",
            sim.name,
            sim.id,
            link(&sim.browser_url)
        );
    }

    Ok(())
}
