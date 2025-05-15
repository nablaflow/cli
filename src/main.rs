use crate::{args::Args, config::Config};
use clap::{CommandFactory, Parser};
use color_eyre::eyre;
use std::io;
use tracing::{Level, debug};

mod aerocloud;
mod args;
mod commands;
mod config;
mod fmt;
mod http;
mod utils;

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

    debug!("args = {:?}", args.scope);

    let config = Config::load(&args).await?;

    match args.scope {
        args::Scope::Config { ref command } => {
            commands::config::run(&args, &config, command).await?;
        }
        args::Scope::AeroCloud { ref command } => {
            commands::aerocloud::run(&args, &config, command).await?;
        }
        args::Scope::GenerateCompletions { shell } => {
            let mut args = Args::command();
            let name = args.get_name().to_string();
            clap_complete::generate(shell, &mut args, name, &mut io::stdout());
        }
        args::Scope::GenerateManpage { dest } => {
            let cmd = Args::command();
            clap_mangen::generate_to(cmd, dest)?;
        }
    }

    Ok(())
}
