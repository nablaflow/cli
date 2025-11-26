use crate::{args::Args, config::Config};
use color_eyre::eyre;

pub async fn run(args: &Args, mut config: Config) -> eyre::Result<()> {
    config.hostname = None;

    config.write(&args.config_path).await
}
