use crate::{
    args::Args,
    config::{Config, Token},
};
use color_eyre::eyre;

pub async fn run(
    args: &Args,
    config: &Config,
    token: &Token,
) -> eyre::Result<()> {
    let mut config = config.clone();
    config.token = Some(token.to_owned());

    config.write(&args.config_path).await
}
