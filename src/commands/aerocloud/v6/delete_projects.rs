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
                        "project_id": id,
                    }))?
                );
            } else {
                println!("Failed to delete project id `{id}`: {err}");
            }
        }
    }

    Ok(())
}

async fn run_one(args: &Args, client: &Client, id: &Id) -> eyre::Result<()> {
    let project = client.projects_v6_get(id).await?.into_inner();

    client.projects_v6_delete(id).await?;

    if !args.json {
        println!("Deleted project `{}` with id {id}", project.name);
    }

    Ok(())
}
