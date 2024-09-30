use crate::{args::Args, config::Config};
use clap::Parser;
use color_eyre::eyre;
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
            args::AeroCloudScope::V6 { ref command } => match command {
                args::AeroCloudV6Command::ListProjects => {
                    commands::aerocloud::v6::list_projects::run(&args, &config)
                        .await?;
                }
            },
        },
        _ => todo!(),
    }

    Ok(())
}
