use crate::{
    aerocloud::{Client, types::User},
    args::Args,
    utils,
};
use color_eyre::eyre;

pub async fn run(args: &Args, client: &Client) -> eyre::Result<()> {
    let user = client.users_self().await?.into_inner();

    if args.json {
        println!("{}", &serde_json::to_string_pretty(&user)?);
    } else {
        print_human(&user);
    }

    Ok(())
}

fn print_human(user: &User) {
    println!(
        "{}",
        utils::new_dynamic_table()
            .set_header(vec!["General"])
            .add_row(vec!["Email", &user.email])
            .add_row(vec![
                "Full name",
                user.full_name.as_ref().map_or("<not set>", |v| v),
            ])
    );

    let Some(ref billing_plan) = user.billing_plan else {
        return;
    };

    let mut table = utils::new_dynamic_table();

    table
        .set_header(vec!["Billing plan"])
        .add_row(vec!["Name", &billing_plan.name])
        .add_row(vec![
            "Available credits",
            &billing_plan.available_credits.to_string(),
        ])
        .add_row(vec!["State", &billing_plan.state.to_string()])
        .add_row(vec![
            "Renewal interval",
            &billing_plan.renewal_interval.to_string(),
        ])
        .add_row(vec![
            "Monthly credits",
            &billing_plan.monthly_credits.to_string(),
        ])
        .add_row(vec!["Started on", &billing_plan.started_on.to_string()]);

    if let Some(ref date) = billing_plan.ends_on {
        table.add_row(vec!["Ends on", &date.to_string()]);
    }
    if let Some(ref date) = billing_plan.renews_on {
        table.add_row(vec!["Renews on", &date.to_string()]);
    }
    if let Some(ref date) = billing_plan.next_monthly_cycle_starts_on {
        table.add_row(vec!["Next monthly cycle starts on", &date.to_string()]);
    }
    if let Some(suspension_reason) = billing_plan.suspension_reason {
        table.add_row(vec!["Suspension reason", &suspension_reason.to_string()]);
    }

    println!("{table}");
}
