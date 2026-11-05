use crate::{
    aerocloud::{
        Client,
        fmt::human_organisation_api_access,
        fmt_progenitor_err,
        types::{ListPageOrganisation, Organisation, PaginationOffset},
    },
    args::Args,
    utils::new_dynamic_table,
};
use chrono::Local;
use color_eyre::eyre;

pub async fn run(
    args: &Args,
    client: &Client,
    show_members: bool,
) -> eyre::Result<()> {
    let all_items = fetch_all(client).await?;

    if args.json {
        println!("{}", serde_json::to_string(&all_items)?);
    } else {
        print_human(&all_items, show_members);
    }

    Ok(())
}

/// Fetches every organisation the current user is a member of.
pub async fn fetch_all(client: &Client) -> eyre::Result<Vec<Organisation>> {
    let mut all_items = vec![];
    let mut offset = PaginationOffset(0u64);

    loop {
        let ListPageOrganisation { items, nav } = client
            .users_self_orgs(None, Some(&offset))
            .await
            .map_err(fmt_progenitor_err)?
            .into_inner();

        all_items.extend(items);

        if let Some(next_offset) = nav.next_offset {
            offset = PaginationOffset(next_offset);
        } else {
            break;
        }
    }

    Ok(all_items)
}

fn print_human(orgs: &[Organisation], show_members: bool) {
    if orgs.is_empty() {
        println!("<empty>");
        return;
    }

    let mut table = new_dynamic_table();
    table.set_header(vec!["Id", "Name", "Members", "API access", "Created at"]);

    for org in orgs {
        table.add_row(vec![
            format!("{}", org.id),
            org.name.clone(),
            org.members.len().to_string(),
            human_organisation_api_access(org.api_access).to_owned(),
            format!("{}", org.created_at.with_timezone(&Local)),
        ]);
    }

    println!("{table}");

    if show_members {
        for org in orgs {
            println!();
            print_members(org);
        }
    }
}

fn print_members(org: &Organisation) {
    println!("Members:");

    if org.members.is_empty() {
        println!("<empty>");
        return;
    }

    let mut table = new_dynamic_table();
    table.set_header(vec!["Id", "Email", "Full name", "Role", "Added at"]);

    for member in &org.members {
        table.add_row(vec![
            format!("{}", member.id),
            member.email.clone(),
            member.full_name.clone().unwrap_or_default(),
            format!("{}", member.role),
            member
                .added_at
                .as_ref()
                .map_or_else(String::new, |added_at| {
                    format!("{}", added_at.with_timezone(&Local))
                }),
        ]);
    }

    println!("{table}");
}
