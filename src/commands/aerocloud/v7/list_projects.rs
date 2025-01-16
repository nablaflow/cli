use crate::{
    args::{Args, Limit, Page},
    config::Config,
    http::{self, format_graphql_errors},
    queries::aerocloud::{
        ProjectStatus, ProjectV7, ProjectsV7Arguments, ProjectsV7Query,
        UnsignedInteger,
    },
};
use color_eyre::eyre::{self, WrapErr};
use cynic::{http::ReqwestExt, QueryBuilder};
use tracing::debug;

pub async fn run(
    args: &Args,
    config: &Config,
    status: Option<ProjectStatus>,
    limit: Limit,
    page: Page,
) -> eyre::Result<()> {
    let (client, endpoint) = http::build_aerocloud_client_from_config(config)?;

    let op_args = ProjectsV7Arguments {
        status,
        limit: UnsignedInteger(limit),
        offset: UnsignedInteger((page.get() - 1).saturating_mul(limit)),
    };
    debug!("args = {op_args:?}");

    let op = ProjectsV7Query::build(op_args);
    debug!("endpoint = {endpoint}");
    debug!("query = {}", op.query);

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to query")?;

    let projects = res
        .data
        .ok_or_else(|| eyre::eyre!(format_graphql_errors(res.errors)))?
        .projects_v7;

    if args.json {
        println!("{}", &serde_json::to_string_pretty(&projects)?);
    } else {
        print_human(&projects);
    }

    Ok(())
}

fn print_human(projects: &[ProjectV7]) {
    let mut table = comfy_table::Table::new();
    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec!["Id", "Name", "Status", "Url"]);

    for project in projects {
        table.add_row(vec![
            format!("{}", project.id.inner()),
            format!("{}", project.name),
            format!("{}", project.status),
            format!("{}", project.browser_url),
        ]);
    }

    println!("{table}");
}
