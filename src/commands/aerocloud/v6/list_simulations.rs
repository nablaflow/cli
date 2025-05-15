use crate::{
    aerocloud::{
        Client,
        types::{
            Fluid, FluidSpeed, Id, ListPageSimulationsV6, PaginationOffset,
            ProjectV6, SimulationQuality, SimulationResultsV6YawAnglesItem,
            SimulationStatus, SimulationV6, SimulationsV6ListStatus, YawAngle,
        },
    },
    args::Args,
    fmt::{NOT_AVAILABLE, link},
};
use chrono::Local;
use color_eyre::eyre;
use itertools::Itertools;
use std::fmt::Write as _;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    args: &Args,
    client: &Client,
    project_id: &Id,
    show_results: bool,
    status: Option<SimulationsV6ListStatus>,
    quality: Option<SimulationQuality>,
    fluid_speed: Option<FluidSpeed>,
    yaw_angle: Option<YawAngle>,
) -> eyre::Result<()> {
    let mut all_items = vec![];
    let mut offset = PaginationOffset(0u64);

    loop {
        let ListPageSimulationsV6 { items, nav } = client
            .simulations_v6_list(
                project_id,
                fluid_speed.as_ref(),
                None,
                Some(&offset),
                quality,
                status,
                yaw_angle.as_ref(),
            )
            .await?
            .into_inner();

        all_items.extend(items);

        if let Some(next_offset) = nav.next_offset {
            offset = PaginationOffset(next_offset);
        } else {
            break;
        }
    }

    if args.json {
        println!("{}", &serde_json::to_string_pretty(&all_items)?);
    } else {
        let project = client.projects_v6_get(project_id).await?.into_inner();

        if show_results {
            print_results_human(&project, &all_items);
        } else {
            print_human(&project, &all_items);
        }
    }

    Ok(())
}

fn print_human(project: &ProjectV6, items: &[SimulationV6]) {
    println!("Project `{}` {}", project.name, link(&project.browser_url));

    if items.is_empty() {
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
            "Status",
            "Quality",
            "Yaw angle(s)",
            "Fluid & Speed",
            "Ground",
            "Created at",
            "",
        ]);

    for sim in items {
        table.add_row(vec![
            format!("{}", sim.name),
            match sim.status {
                SimulationStatus::Progress => "🚧".into(),
                SimulationStatus::Success => "✅".into(),
                SimulationStatus::QualityCheck => "🔍".into(),
                SimulationStatus::Expired => "♽".into(),
            },
            format!("{}", sim.params.quality),
            sim.params
                .yaw_angles
                .iter()
                .map(|v| format!("{v}°"))
                .join(", "),
            format!("{}, {} m/s", sim.params.fluid, sim.params.fluid_speed),
            if let (Fluid::Air, true) = (sim.params.fluid, sim.params.has_ground)
            {
                let mut s = format!(
                    "present, {}",
                    if sim.params.is_ground_moving {
                        "moving"
                    } else {
                        "still"
                    },
                );

                if sim.params.ground_offset.0 != 0.0 {
                    let _ =
                        write!(s, ", offset: {:.2} m", sim.params.ground_offset);
                }

                s
            } else {
                NOT_AVAILABLE.into()
            },
            format!("{}", sim.created_at.with_timezone(&Local)),
            link(&sim.browser_url),
        ]);
    }

    println!("{table}");
}

fn print_results_human(project: &ProjectV6, items: &[SimulationV6]) {
    println!(
        "Project results: `{}` {}",
        project.name,
        link(&project.browser_url)
    );

    let items: Vec<(&SimulationV6, &SimulationResultsV6YawAnglesItem)> = items
        .iter()
        .filter_map(|sim| sim.results.as_ref().map(|results| (sim, results)))
        .flat_map(|(sim, results)| {
            results
                .yaw_angles
                .iter()
                .map(move |yaw_angle_results| (sim, yaw_angle_results))
        })
        .collect();

    if items.is_empty() {
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
            "Fluid & Speed",
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
            "Heat transfer",
            "Heat transfer coeff",
        ]);

    for (sim, res) in items {
        table.add_row(vec![
            format!("{}", sim.name),
            format!("{}", sim.params.quality),
            format!("{}°", res.yaw_angle),
            format!("{}, {} m/s", sim.params.fluid, sim.params.fluid_speed),
            res.surface_area
                .map_or(NOT_AVAILABLE.into(), |v| format!("{v:.2} m²")),
            format!("{:.2} N", res.fd),
            format!("{:.2} N", res.fl),
            format!("{:.2} N", res.fs),
            format!("{:.2}", res.cd),
            format!("{:.2}", res.cl),
            format!("{:.2}", res.cs),
            format!("{:.2} m²", res.cda),
            format!("{:.2} m²", res.cla),
            format!("{:.2} m²", res.csa),
            format!("{:.2} Nm", res.mr),
            format!("{:.2} Nm", res.my),
            format!("{:.2} Nm", res.mp),
            res.heat_transfer
                .map_or(NOT_AVAILABLE.into(), |v| format!("{v:.2} W/K")),
            res.heat_transfer_coefficient
                .map_or(NOT_AVAILABLE.into(), |v| format!("{v:.2} W/m²K")),
        ]);
    }

    for col in table.column_iter_mut().skip(2) {
        col.set_cell_alignment(comfy_table::CellAlignment::Right);
    }

    println!("{table}");
}
