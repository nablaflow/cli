use crate::{
    aerocloud::{
        Client,
        types::{ListPageProjectsV7, PaginationOffset, ProjectStatus, ProjectV7},
    },
    args::Args,
    fmt::link,
};
use chrono::Local;
use color_eyre::eyre;

pub async fn run(
    args: &Args,
    client: &Client,
    status: Option<ProjectStatus>,
) -> eyre::Result<()> {
    let mut all_items = vec![];
    let mut offset = PaginationOffset(0u64);

    loop {
        let ListPageProjectsV7 { items, nav } = client
            .projects_v7_list(None, Some(&offset), status)
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
        println!("{}", &serde_json::to_string(&all_items)?);
    } else {
        print_human(&all_items);
    }

    Ok(())
}

fn print_human(projects: &[ProjectV7]) {
    if projects.is_empty() {
        println!("<empty>");
        return;
    }

    let mut table = comfy_table::Table::new();
    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec!["Id", "Name", "Status", "Created at", ""]);

    for project in projects {
        table.add_row(vec![
            format!("{}", project.id),
            format!("{}", project.name),
            format!("{}", project.status),
            format!("{}", project.created_at.with_timezone(&Local)),
            link(&project.browser_url),
        ]);
    }

    println!("{table}");
}
