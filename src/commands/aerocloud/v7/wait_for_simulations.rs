use crate::{
    aerocloud::{
        Client,
        types::{Id, SimulationStatus},
    },
    args::Args,
    fmt::link,
};
use color_eyre::eyre::{self, bail};
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub async fn run(args: &Args, client: &Client, ids: &[Id]) -> eyre::Result<()> {
    if ids.is_empty() {
        bail!("No ids were specified!");
    }

    let mut ids_to_wait_for: Vec<Uuid> =
        ids.iter().map(|id| **id).collect::<Vec<_>>();

    if !args.json {
        info!("Subscribed! Waiting for events...");
    }

    while !ids_to_wait_for.is_empty() {
        let mut found_ids = vec![];

        for id in &ids_to_wait_for {
            debug!("Polling sim `{id}`...");

            match client.simulations_v7_get(&Id(*id)).await {
                Ok(res) => {
                    let sim = res.into_inner();

                    if let SimulationStatus::Success | SimulationStatus::Expired =
                        sim.status
                    {
                        found_ids.push(*id);

                        if args.json {
                            println!("{}", serde_json::to_string(&sim)?);
                        } else {
                            info!(
                                "Simulation `{}` has completed. {}",
                                sim.id,
                                link(&sim.browser_url)
                            );
                        }
                    } else {
                        debug!("Sim `{id}` still in progress...");
                    }
                }
                Err(err) => {
                    warn!("Failed to poll sim `{id}`: {err}. Will retry later.");
                }
            }
        }

        ids_to_wait_for.retain(|id| !found_ids.contains(id));

        tokio::time::sleep(Duration::from_secs(60)).await;
    }

    Ok(())
}
