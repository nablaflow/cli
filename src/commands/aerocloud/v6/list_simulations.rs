use crate::{
    args::Args,
    config::Config,
    http::{self, format_graphql_errors},
    queries::aerocloud::{
        ProjectV6WithSimulations, SimulationQuality,
        SimulationsInProjectV6Arguments, SimulationsInProjectV6Query, Speed,
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
    quality: Option<SimulationQuality>,
    speed: Option<f32>,
) -> eyre::Result<()> {
    let (client, endpoint) = http::build_aerocloud_client_from_config(config)?;

    let op_args = SimulationsInProjectV6Arguments {
        id: Id::new(project_id),
        quality,
        speed: speed.map(Speed),
    };
    debug!("args = {:#?}", op_args);

    let op = SimulationsInProjectV6Query::build(op_args);
    debug!("endpoint = {endpoint}");
    debug!("query = {}", op.query);

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to query")?;

    let Some(project) = res
        .data
        .ok_or_else(|| eyre::eyre!(format_graphql_errors(res.errors)))?
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
    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Name",
            "Quality",
            "Yaw angle(s)",
            "Status",
            "Created at",
            "Url",
        ]);

    for sim in &project.simulations {
        table.add_row(vec![
            format!("{}", sim.name),
            format!("{}", sim.inputs.quality),
            sim.inputs
                .yaw_angles
                .iter()
                .map(|yaw_angle| format!("{yaw_angle}"))
                .join(", "),
            format!("{}", sim.status),
            format!("{}", sim.created_at),
            format!("{}", sim.browser_url),
        ]);
    }

    println!("{table}");
}
