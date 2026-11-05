use crate::{
    aerocloud::{Client, types::Id},
    args::Args,
    commands::aerocloud::org,
    config::{Config, ConfigOrg},
};
use color_eyre::eyre::{self, WrapErr};
use tracing::info;

pub async fn run(args: &Args, client: &Client, org_id: &Id) -> eyre::Result<()> {
    let org = org::list::fetch_all(client)
        .await
        .wrap_err("fetching organisations")?
        .into_iter()
        .find(|org| *org.id == **org_id)
        .ok_or_else(|| {
            eyre::eyre!(
                "you are not a member of organisation `{org_id}`, see `nf aerocloud org list`"
            )
        })?;

    let org = ConfigOrg {
        id: org.id,
        name: org.name,
    };

    // NOTE: only what is on disk, so overrides from flags or the env do not get saved.
    let mut config = Config::load_from_path(&args.config_path).await?;
    config.aerocloud_org = Some(org.clone());
    config.write(&args.config_path).await?;

    info!("saved default organisation {org} in config");

    Ok(())
}
