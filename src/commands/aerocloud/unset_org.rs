use crate::{args::Args, config::Config};
use color_eyre::eyre;

pub async fn run(args: &Args) -> eyre::Result<()> {
    let mut config = Config::load_from_path(&args.config_path).await?;
    config.aerocloud_org = None;

    config.write(&args.config_path).await
}
