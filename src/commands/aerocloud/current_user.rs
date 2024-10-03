use crate::{
    args::Args,
    config::Config,
    http,
    queries::aerocloud::{User, ViewerQuery},
};
use color_eyre::eyre::{self, WrapErr};
use cynic::{http::ReqwestExt, QueryBuilder};
use tracing::debug;

pub async fn run(args: &Args, config: &Config) -> eyre::Result<()> {
    let (client, endpoint) = http::build_aerocloud_client_from_config(config)?;

    let op = ViewerQuery::build(());

    debug!("{endpoint}, {}", op.query);

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to query")?;

    let user = res
        .data
        .ok_or_else(|| eyre::eyre!("bad response"))?
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
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    table.set_header(vec!["General"]);
    table.add_row(vec!["Id", user.id.inner()]);
    table.add_row(vec!["Email", &user.email]);
    table.add_row(vec![
        "Full name",
        user.full_name.as_ref().map_or("<not set>", |v| v),
    ]);
    println!("{table}");

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    table.set_header(vec!["Billing"]);
    table.add_row(vec![
        "Current plan",
        &format!("{:?}", user.billing.current_plan),
    ]);
    table.add_row(vec![
        "Total available credits",
        &user.billing.total_available_credits.to_string(),
    ]);
    println!("{table}");

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    table.set_header(vec!["Purchased credits"]);
    table.add_row(vec!["Amount", &user.billing.purchased_credits.to_string()]);
    if let Some(date) = user.billing.purchased_credits_expire_on {
        table.add_row(vec!["Expire on", &date.to_string()]);
    }
    println!("{table}");

    if let Some(subscription) = &user.billing.subscription {
        let mut table = comfy_table::Table::new();
        table.load_preset(comfy_table::presets::UTF8_FULL);
        table.apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

        table.set_header(vec!["Subscription"]);
        table.add_row(vec!["State", &format!("{:?}", subscription.state)]);
        table.add_row(vec!["Interval", &format!("{:?}", subscription.interval)]);
        table.add_row(vec![
            "Monthly credits",
            &subscription.monthly_credits.to_string(),
        ]);
        table.add_row(vec![
            "Available credits",
            &subscription.monthly_credits.to_string(),
        ]);
        table.add_row(vec!["Started on", &subscription.started_on.to_string()]);

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
                &format!("{:?}", suspension_reason),
            ]);
        }
        println!("{table}");
    }
}
