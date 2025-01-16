use crate::{
    args::Args,
    config::Config,
    fmt::display_optional,
    http::{self, format_graphql_errors},
    queries::aerocloud::{
        ProjectV6WithSimulationsResults, SimulationQuality,
        SimulationsWithResultsInProjectV6Arguments,
        SimulationsWithResultsInProjectV6Query, Speed, YawAngle,
    },
};
use color_eyre::eyre::{self, bail, WrapErr};
use cynic::{http::ReqwestExt, Id, QueryBuilder};
use tracing::debug;

pub async fn run(
    args: &Args,
    config: &Config,
    project_id: &str,
    quality: Option<SimulationQuality>,
    speed: Option<f32>,
    yaw_angles: Option<&[f32]>,
) -> eyre::Result<()> {
    let (client, endpoint) = http::build_aerocloud_client_from_config(config)?;

    let op_args = SimulationsWithResultsInProjectV6Arguments {
        id: Id::new(project_id),
        quality,
        speed: speed.map(Speed),
        yaw_angles: yaw_angles.map(|v| v.iter().copied().map(YawAngle).collect()),
    };
    debug!("args = {:#?}", op_args);

    let op = SimulationsWithResultsInProjectV6Query::build(op_args);

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

fn print_human(project: &ProjectV6WithSimulationsResults) {
    println!("Project `{}` with results:", project.name);

    if project.simulations.is_empty()
        || project
            .simulations
            .iter()
            .filter_map(|sim| sim.results.as_ref())
            .filter_map(|res| res.yaw_angles.as_ref())
            .flatten()
            .count()
            == 0
    {
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
            "Yaw angle",
            "Speed",
            "Surface",
            "Fd",
            "Fl",
            "Fs",
            "Cd",
            "Cl",
            "Cs",
            "Cda",
            "Cla",
            "Csa",
            "Mr",
            "My",
            "Mp",
            "Heat transfer value",
            "Heat transfer coefficient",
        ]);

    for sim in &project.simulations {
        if let Some(results) = &sim.results {
            for yaw_angle_results in
                results.yaw_angles.as_ref().unwrap_or(&Vec::new())
            {
                table.add_row(vec![
                    format!("{}", sim.name),
                    format!("{}", sim.inputs.quality),
                    format!("{}", yaw_angle_results.degrees),
                    format!("{}", sim.inputs.speed),
                    display_optional(yaw_angle_results.surface.as_ref()),
                    format!("{}", yaw_angle_results.force.fd),
                    format!("{}", yaw_angle_results.force.fl),
                    format!("{}", yaw_angle_results.force.fs),
                    format!("{}", yaw_angle_results.coefficient.cd),
                    format!("{}", yaw_angle_results.coefficient.cl),
                    format!("{}", yaw_angle_results.coefficient.cs),
                    format!("{}", yaw_angle_results.coefficient_area.cda),
                    format!("{}", yaw_angle_results.coefficient_area.cla),
                    format!("{}", yaw_angle_results.coefficient_area.csa),
                    format!("{}", yaw_angle_results.moment.mr),
                    format!("{}", yaw_angle_results.moment.my),
                    format!("{}", yaw_angle_results.moment.mp),
                    display_optional(
                        yaw_angle_results.heat_transfer.value.as_ref(),
                    ),
                    display_optional(
                        yaw_angle_results.heat_transfer.coefficient.as_ref(),
                    ),
                ]);
            }
        }
    }

    for col in table.column_iter_mut().skip(2) {
        col.set_cell_alignment(comfy_table::CellAlignment::Right);
    }

    println!("{table}");
}
