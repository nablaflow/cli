use crate::{
    aerocloud::{
        new_idempotency_key, types::CreateProjectV7Params, v7::V7Client,
    },
    args::Args,
};
use color_eyre::eyre;

pub async fn run(
    args: &Args,
    client: &V7Client,
    name: &str,
    description: Option<&str>,
) -> eyre::Result<()> {
    let idempotency_key = new_idempotency_key();

    let project = client
        .projects_v7_create(
            &idempotency_key,
            &CreateProjectV7Params {
                name: name.into(),
                description: description.map(ToOwned::to_owned),
                members: vec![],
            },
        )
        .await?
        .into_inner();

    if args.json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "project_id": project.id,
            }))?
        );
    } else {
        println!("Created project `{name}` with id {}\n", project.id);
        println!("Browser url: {}", project.browser_url);
    }

    Ok(())
}
