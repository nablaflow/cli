use crate::{
    args::Args,
    config::Config,
    http::{self, format_graphql_errors},
    queries::aerocloud::SimulationV6SucceededSubscription,
};
use color_eyre::eyre::{self, WrapErr};
use cynic::{Id, SubscriptionBuilder};
use futures_util::StreamExt;
use std::collections::HashSet;
use tracing::{debug, info, warn};

pub async fn run(
    args: &Args,
    config: &Config,
    ids: &[String],
) -> eyre::Result<()> {
    let (client, endpoint) =
        http::build_aerocloud_ws_client_from_config(config).await?;

    debug!("endpoint = {endpoint}");

    let op = SimulationV6SucceededSubscription::build(());

    let mut stream = client
        .subscribe(op)
        .await
        .wrap_err("subscribing to endpoint")?;

    let mut ids_to_wait_for = ids.iter().map(Id::new).collect::<HashSet<_>>();

    if !args.json {
        if ids_to_wait_for.is_empty() {
            warn!("No ids were specified, will never exit!");
        }

        info!("Subscribed! Waiting for events...");
    }

    while let Some(res) = stream.next().await {
        let inner_res = res.wrap_err("reading next item")?;
        let sim = inner_res
            .data
            .ok_or_else(|| eyre::eyre!(format_graphql_errors(inner_res.errors)))?
            .simulation_v6_succeeded;

        if !ids_to_wait_for.contains(&sim.id) {
            continue;
        }

        ids_to_wait_for.remove(&sim.id);

        if args.json {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "simulation_id": sim.id,
                    "browser_url": sim.browser_url,
                }))?
            );
        } else {
            info!(
                "Simulation `{}` has been published. You can see results at {}",
                sim.id.inner(),
                sim.browser_url
            );
        }

        if ids_to_wait_for.is_empty() {
            info!("All ids were waited for, exiting.");
            break;
        }
    }

    Ok(())
}
