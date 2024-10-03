use crate::{
    args::Args,
    config::Config,
    http,
    queries::aerocloud::{
        ProjectV6WithSimulations, SimulationsInProjectV6Arguments,
        SimulationsInProjectV6Query,
    },
};
use color_eyre::eyre::{self, bail, WrapErr};
use cynic::{http::ReqwestExt, Id, QueryBuilder};
use itertools::Itertools;
use tracing::debug;

pub async fn run(
    args: &Args,
    config: &Config,
    project_id: &str,
) -> eyre::Result<()> {
    let (client, endpoint) = http::build_aerocloud_client_from_config(config)?;

    let id = Id::new(project_id);
    let op =
        SimulationsInProjectV6Query::build(SimulationsInProjectV6Arguments {
            id,
        });

    debug!("{endpoint}, {}", op.query);

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to query")?;

    let Some(project) = res
        .data
        .ok_or_else(|| eyre::eyre!("bad response"))?
        .project_v6
    else {
        bail!("project Id {project_id} not found");
    };

    if args.json {
        println!("{}", &serde_json::to_string_pretty(&project.simulations)?);
    } else {
        print_human(&project);
    }

    Ok(())
}

fn print_human(project: &ProjectV6WithSimulations) {
    println!("Project: {}", project.name);

    if project.simulations.is_empty() {
        println!("\n<empty>");
        return;
    }

    let mut table = comfy_table::Table::new();

    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    table.set_header(vec![
        "Id",
        "Name",
        "Quality",
        "Yaw angle(s)",
        "Status",
        "Created at",
        "Url",
    ]);

    for sim in &project.simulations {
        table.add_row(vec![
            format!("{}", sim.id.inner()),
            format!("{}", sim.name),
            format!("{:?}", sim.inputs.quality),
            sim.inputs.yaw_angles.iter().map(|n| n.0).join(", "),
            format!("{:?}", sim.status),
            format!("{}", sim.created_at),
            format!("{}", sim.browser_url),
        ]);
    }

    println!("{table}");
}
