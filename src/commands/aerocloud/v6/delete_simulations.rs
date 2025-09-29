use crate::{
    aerocloud::{Client, types::Id},
    args::Args,
};
use color_eyre::eyre;
use serde_json::json;

pub async fn run(args: &Args, client: &Client, ids: &[Id]) -> eyre::Result<()> {
    for id in ids {
        if let Err(err) = run_one(args, client, id).await {
            if args.json {
                println!(
                    "{}",
                    &serde_json::to_string(&json!({
                        "error": format!("{}", err),
                        "simulation_id": id,
                    }))?
                );
            } else {
                println!("Failed to delete simulation with id {id}: {err}");
            }
        }
    }

    Ok(())
}

async fn run_one(args: &Args, client: &Client, id: &Id) -> eyre::Result<()> {
    let simulation = client.simulations_v6_get(id).await?.into_inner();

    client.simulations_v6_delete(id).await?;

    if !args.json {
        println!("Deleted simulation `{}` with id {id}", simulation.name);
    }

    Ok(())
}
