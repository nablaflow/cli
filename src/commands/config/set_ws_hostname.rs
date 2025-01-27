use crate::{args::Args, config::Config};
use color_eyre::eyre;
use reqwest::Url;

pub async fn run(
    args: &Args,
    config: &Config,
    hostname: &Url,
) -> eyre::Result<()> {
    let mut config = config.clone();
    config.ws_hostname = Some(hostname.to_owned());

    config.write(&args.config_path).await
}
