use clap::{Parser, Subcommand};
use reqwest::Url;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, env = "NF_DEBUG")]
    pub debug: bool,

    #[arg(short, long, env = "NF_JSON")]
    pub json: bool,

    #[arg(
        short = 'H',
        long,
        default_value = "https://api.nablaflow.io",
        value_name = "URI",
        help = "Host to connect to",
        env = "NF_HOSTNAME"
    )]
    pub hostname: Url,

    #[arg(
        short,
        long,
        env = "NF_TOKEN",
        value_name = "TOKEN",
        help = "Token to use during requests. If specified will take precedence over the one set in config"
    )]
    pub token: Option<String>,

    #[arg(
        short,
        long,
        default_value_os_t = crate::config::path().expect("failed to determine default config dir"),
        value_name = "CONFIGPATH",
        help = "CLI configuration file",
        env = "NF_CONFIG",
    )]
    pub config: PathBuf,

    #[command(subcommand)]
    pub scope: Scope,
}

#[derive(Subcommand, Debug)]
pub enum Scope {
    #[command(name = "authtoken")]
    AuthToken {
        #[arg(
            env = "NF_TOKEN",
            value_name = "TOKEN",
            help = "Token to set in config"
        )]
        token: String,
    },
    #[command(name = "aerocloud")]
    AeroCloud {
        #[command(subcommand)]
        command: AeroCloudScope,
    },
}

#[derive(Subcommand, Debug)]
pub enum AeroCloudScope {
    CurrentUser,
    V6 {
        #[command(subcommand)]
        command: AeroCloudV6Command,
    },
}

#[derive(Subcommand, Debug)]
pub enum AeroCloudV6Command {
    ListProjects,
}
