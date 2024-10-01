use crate::{args::Args, config::Config};
use clap::Parser;
use color_eyre::eyre::{self, WrapErr};
use tracing::Level;

mod args;
mod commands;
mod config;
mod http;
mod queries;

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(if args.debug {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .init();

    let token = args
        .token
        .clone()
        .ok_or_else(|| eyre::eyre!("no token provided"))?;

    let config = Config {
        token,
        hostname: args.hostname.clone(),
    };

    match args.scope {
        args::Scope::AeroCloud { ref command } => match command {
            args::AeroCloudScope::CurrentUser => {
                commands::aerocloud::current_user::run(&args, &config).await?;
            }
            args::AeroCloudScope::V6 { command } => match command {
                args::AeroCloudV6Command::ListProjects => {
                    commands::aerocloud::v6::list_projects::run(&args, &config)
                        .await?;
                }
                args::AeroCloudV6Command::ListSimulations { project_id } => {
                    commands::aerocloud::v6::list_simulations::run(
                        &args, &config, project_id,
                    )
                    .await?;
                }
                args::AeroCloudV6Command::CreateModel { params } => {
                    commands::aerocloud::v6::create_model::run(
                        &args,
                        &config,
                        &params
                            .clone()
                            .contents()
                            .wrap_err("failed to read contents")?,
                    )
                    .await?;
                }
            },
        },
        _ => todo!(),
    }

    Ok(())
}
