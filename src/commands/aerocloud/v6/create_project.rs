use crate::{
    args::Args,
    config::Config,
    http::{self, format_graphql_errors},
    queries::aerocloud::{
        CreateProjectV6Mutation, CreateProjectV6MutationParams, InputProjectV6,
    },
};
use color_eyre::eyre::{self, WrapErr};
use cynic::{http::ReqwestExt, MutationBuilder};
use tracing::debug;

pub async fn run(
    args: &Args,
    config: &Config,
    name: &str,
    description: Option<&str>,
) -> eyre::Result<()> {
    let (client, endpoint) = http::build_aerocloud_client_from_config(config)?;

    let op_params = CreateProjectV6MutationParams {
        input: InputProjectV6 {
            name: name.into(),
            description: description.map(Into::into),
        },
    };
    debug!("args = {op_params:?}");

    let op = CreateProjectV6Mutation::build(op_params);
    debug!("endpoint = {endpoint}");
    debug!("query = {}", op.query);

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to query")?;

    let project = res
        .data
        .ok_or_else(|| eyre::eyre!(format_graphql_errors(res.errors)))?
        .create_project_v6;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "project_id": project.id,
            }))?
        );
    } else {
        println!("Created project `{name}` with id {}\n", project.id.inner());
        println!("Browser url: {}", project.browser_url);
    }

    Ok(())
}
