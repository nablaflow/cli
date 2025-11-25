use crate::{
    aerocloud::{
        Client, fmt_progenitor_err, new_idempotency_key,
        types::CreateProjectV6Params,
    },
    args::Args,
};
use color_eyre::eyre;

pub async fn run(
    args: &Args,
    client: &Client,
    name: &str,
    description: Option<&str>,
) -> eyre::Result<()> {
    let idempotency_key = new_idempotency_key();

    let project = client
        .projects_v6_create(
            &idempotency_key,
            &CreateProjectV6Params {
                name: name.into(),
                description: description.map(ToOwned::to_owned),
            },
        )
        .await
        .map_err(fmt_progenitor_err)?
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
