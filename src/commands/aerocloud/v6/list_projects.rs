use crate::{
    args::Args,
    config::Config,
    http,
    queries::aerocloud::{ProjectV6, ProjectsV6Query},
};
use color_eyre::eyre::{self, WrapErr};
use cynic::{http::ReqwestExt, QueryBuilder};
use tracing::debug;

pub async fn run(args: &Args, config: &Config) -> eyre::Result<()> {
    let (client, endpoint) =
        http::build_aerocloud_client(&config.token, &config.hostname)?;
    let op = ProjectsV6Query::build(());
    debug!("{endpoint}, {}", op.query);

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to query")?;

    let projects = res
        .data
        .ok_or_else(|| eyre::eyre!("bad response"))?
        .projects_v6;

    if args.json {
        println!("{}", &serde_json::to_string_pretty(&projects)?);
    } else {
        print_human(&projects);
    }

    Ok(())
}

fn print_human(projects: &[ProjectV6]) {
    let mut table = comfy_table::Table::new();

    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    table.set_header(vec!["Id", "Name", "Url"]);

    for project in projects {
        table.add_row(vec![
            format!("{}", project.id.inner()),
            format!("{}", project.name),
            format!("{}", project.browser_url),
        ]);
    }

    println!("{table}");
}
