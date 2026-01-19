use crate::{args::Args, config::Config};
use clap::{CommandFactory, Parser};
use color_eyre::eyre::{self, WrapErr};
use std::io;
use update_informer::{Check, registry};

mod aerocloud;
mod args;
mod commands;
mod config;
mod fmt;
mod http;
mod tracing;
mod utils;

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    crate::tracing::init(&args).wrap_err("initializing tracing")?;

    if !(args.json || args.skip_update_check) {
        check_for_new_version();
    }

    let config = Config::load(&args).await?;

    match args.scope {
        args::Scope::Config { ref command } => {
            commands::config::run(&args, config, command).await?;
        }
        args::Scope::AeroCloud { ref command } => {
            commands::aerocloud::run(&args, config, command).await?;
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

fn check_for_new_version() {
    let current_version = env!("CARGO_PKG_VERSION");

    let informer =
        update_informer::new(registry::GitHub, "nablaflow/cli", current_version);

    if let Ok(Some(new_version)) = informer.check_version() {
        eprintln!(
            "⚡️ A new release is available: v{current_version} -> {new_version}. See https://github.com/nablaflow/cli/releases. ⚡️\n"
        );
    }
}
