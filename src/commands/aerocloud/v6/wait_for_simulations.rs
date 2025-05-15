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

    loop {
        info!(
            "waiting for {} simulation(s) to complete...",
            ids_to_wait_for.len()
        );

        let mut found_ids = vec![];

        for id in &ids_to_wait_for {
            debug!("polling sim `{id}`...");

            match client.simulations_v6_get(&Id(*id)).await {
                Ok(res) => {
                    let sim = res.into_inner();

                    if let SimulationStatus::Success | SimulationStatus::Expired =
                        sim.status
                    {
                        found_ids.push(*id);

                        if args.json {
                            println!("{}", serde_json::to_string(&sim)?);
                        } else {
                            println!(
                                "Simulation `{}` has completed. {}",
                                sim.id,
                                link(&sim.browser_url)
                            );
                        }
                    } else {
                        debug!("sim `{id}` still in progress...");
                    }
                }
                Err(err) => {
                    warn!("failed to poll sim `{id}`: {err}. will retry later.");
                }
            }
        }

        ids_to_wait_for.retain(|id| !found_ids.contains(id));

        if ids_to_wait_for.is_empty() {
            break;
        }

        tokio::time::sleep(Duration::from_secs(60)).await;
    }

    Ok(())
}
