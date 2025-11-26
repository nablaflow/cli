use crate::{args::Args, config::Config};
use color_eyre::eyre;
use reqwest::Url;

pub async fn run(
    args: &Args,
    mut config: Config,
    hostname: &Url,
) -> eyre::Result<()> {
    config.hostname = Some(hostname.to_owned());

    config.write(&args.config_path).await
}
