use crate::{
    aerocloud::{
        Client, new_idempotency_key,
        types::{CreateSimulationV6Params, Id},
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

    let mut params = serde_json::from_str::<CreateSimulationV6Params>(params)
        .wrap_err("failed to parse json")?;

    if let Some(id) = model_id {
        params.model_id = id;
    }
    if let Some(id) = project_id {
        params.project_id = id;
    }

    let sim = client
        .simulations_v6_create(&idempotency_key, &params)
        .await?
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
