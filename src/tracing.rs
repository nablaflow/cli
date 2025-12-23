use crate::args::Args;
use color_eyre::eyre::{self, WrapErr};
use std::{fs::OpenOptions, io};
use tracing::Level;

pub fn init(args: &Args) -> eyre::Result<()> {
    let max_level = if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    if let Some(ref path) = args.log_to_path {
        tracing_subscriber::fmt()
            .with_writer(
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(path)
                    .wrap_err("opening log file")?,
            )
            .with_max_level(max_level)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_writer(io::stderr)
            .with_max_level(max_level)
            .init();
    }

    Ok(())
}
