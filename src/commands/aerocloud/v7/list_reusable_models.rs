use crate::{
    aerocloud::{
        Client,
        types::{ListPageModelsV7, ModelV7, PaginationOffset},
    },
    args::Args,
    utils::new_dynamic_table,
};
use chrono::Local;
use color_eyre::eyre;

pub async fn run(args: &Args, client: &Client) -> eyre::Result<()> {
    let mut all_items = vec![];
    let mut offset = PaginationOffset(0u64);

    loop {
        let ListPageModelsV7 { items, nav } = client
            .models_v7_list_reusable(None, Some(&offset))
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

fn print_human(models: &[ModelV7]) {
    if models.is_empty() {
        println!("<empty>");
        return;
    }

    let mut table = new_dynamic_table();
    table.set_header(vec!["Id", "Name", "Files", "Parts", "Created at"]);

    for model in models {
        table.add_row(vec![
            format!("{}", model.id),
            format!("{}", model.name),
            format!("{}", model.files.len()),
            format!(
                "{}",
                model.files.iter().map(|f| f.parts.len()).sum::<usize>()
            ),
            format!("{}", model.created_at.with_timezone(&Local)),
        ]);
    }

    println!("{table}");
}
