use crate::{
    args::Args,
    config::Config,
    http::{self, format_graphql_errors},
    queries::aerocloud::{User, ViewerQuery},
};
use color_eyre::eyre::{self, WrapErr};
use cynic::{http::ReqwestExt, QueryBuilder};
use tracing::debug;

pub async fn run(args: &Args, config: &Config) -> eyre::Result<()> {
    let (client, endpoint) = http::build_aerocloud_client_from_config(config)?;

    let op = ViewerQuery::build(());
    debug!("endpoint = {endpoint}");
    debug!("query = {}", op.query);

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to query")?;

    let user = res
        .data
        .ok_or_else(|| eyre::eyre!(format_graphql_errors(res.errors)))?
        .viewer
        .ok_or_else(|| eyre::eyre!("bad response"))?;

    if args.json {
        println!("{}", &serde_json::to_string_pretty(&user)?);
    } else {
        print_human(&user);
    }

    Ok(())
}

fn print_human(user: &User) {
    let mut table = comfy_table::Table::new();
    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec!["General"])
        .add_row(vec!["Email", &user.email])
        .add_row(vec![
            "Full name",
            user.full_name.as_ref().map_or("<not set>", |v| v),
        ]);
    println!("{table}");

    let mut table = comfy_table::Table::new();
    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec!["Billing"])
        .add_row(vec![
            "Current plan",
            &format!("{}", user.billing.current_plan),
        ])
        .add_row(vec![
            "Total available credits",
            &user.billing.total_available_credits.to_string(),
        ]);
    println!("{table}");

    let mut table = comfy_table::Table::new();

    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec!["Purchased credits"])
        .add_row(vec!["Amount", &user.billing.purchased_credits.to_string()]);

    if let Some(date) = user.billing.purchased_credits_expire_on {
        table.add_row(vec!["Expire on", &date.to_string()]);
    }

    println!("{table}");

    if let Some(subscription) = &user.billing.subscription {
        let mut table = comfy_table::Table::new();

        table
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec!["Subscription"])
            .add_row(vec!["State", &format!("{}", subscription.state)])
            .add_row(vec!["Interval", &format!("{}", subscription.interval)])
            .add_row(vec![
                "Monthly credits",
                &subscription.monthly_credits.to_string(),
            ])
            .add_row(vec![
                "Available credits",
                &subscription.monthly_credits.to_string(),
            ])
            .add_row(vec!["Started on", &subscription.started_on.to_string()]);

        if let Some(date) = subscription.ends_on {
            table.add_row(vec!["Ends on", &date.to_string()]);
        }
        if let Some(date) = subscription.renews_on {
            table.add_row(vec!["Renews on", &date.to_string()]);
        }
        if let Some(date) = subscription.next_monthly_cycle_starts_on {
            table
                .add_row(vec!["Next monthly cycle starts on", &date.to_string()]);
        }
        if let Some(suspension_reason) = subscription.suspension_reason {
            table.add_row(vec![
                "Suspension reason",
                &format!("{suspension_reason}"),
            ]);
        }

        println!("{table}");
    }
}
