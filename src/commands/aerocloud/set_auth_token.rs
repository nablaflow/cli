use crate::{
    args::Args,
    commands::aerocloud::current_token,
    config::{Config, Token},
    http::build_aerocloud_client_from_config,
};
use color_eyre::eyre::{self, WrapErr};
use tracing::info;

pub async fn run(
    args: &Args,
    mut config: Config,
    token: &Token,
) -> eyre::Result<()> {
    info!("validating token...");

    config.aerocloud_token = Some(token.to_owned());

    let client =
        build_aerocloud_client_from_config(&config, &args.http_timeout())?;

    current_token::run(args, &client)
        .await
        .wrap_err("validating token")?;

    config.write(&args.config_path).await?;

    info!("saved valid token in config");

    Ok(())
}
