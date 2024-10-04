use crate::{args::Args, config::Config};
use color_eyre::eyre;

pub async fn run(args: &Args, config: &Config) -> eyre::Result<()> {
    let mut config = config.clone();
    config.hostname = None;

    config.write(&args.config_path).await
}
