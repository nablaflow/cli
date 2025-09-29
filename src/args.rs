use crate::{
    aerocloud::types::{
        FluidSpeed, Id, ProjectStatus, SimulationQuality,
        SimulationsV6ListStatus, SimulationsV7ListStatus, YawAngle,
    },
    config::{Config, Token},
};
use clap::{Parser, Subcommand};
use clap_complete::aot::Shell;
use clap_stdin::{FileOrStdin, MaybeStdin};
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
        value_name = "URI",
        help = "Host to connect to. If specified will take precedence over the one set in config",
        env = "NF_HOSTNAME"
    )]
    pub hostname: Option<Url>,

    #[arg(
        short,
        long,
        env = "NF_TOKEN",
        value_name = "TOKEN",
        help = "Token to use during requests. If specified will take precedence over the one set in config"
    )]
    pub token: Option<Token>,

    #[arg(
        short,
        long,
        default_value_os_t = Config::default_path().expect("failed to determine default config dir"),
        value_name = "CONFIGPATH",
        help = "CLI configuration file",
        env = "NF_CONFIGPATH",
    )]
    pub config_path: PathBuf,

    #[command(subcommand)]
    pub scope: Scope,
}

#[derive(Subcommand, Debug)]
pub enum Scope {
    Config {
        #[command(subcommand)]
        command: ConfigScope,
    },
    #[command(name = "aerocloud")]
    AeroCloud {
        #[command(subcommand)]
        command: AeroCloudScope,
    },
    GenerateCompletions {
        shell: Shell,
    },
    GenerateManpage {
        dest: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum AeroCloudScope {
    CurrentUser,
    V6 {
        #[command(subcommand)]
        command: AeroCloudV6Command,
    },
    V7 {
        #[command(subcommand)]
        command: AeroCloudV7Command,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigScope {
    SetAuthToken {
        #[arg(value_name = "TOKEN", help = "Token to set in config")]
        token: MaybeStdin<Token>,
    },
    SetHostname {
        #[arg(value_name = "HOSTNAME", help = "Hostname to set in config")]
        hostname: Url,
    },
    UnsetHostname,
    Show {
        #[arg(
            short = 's',
            long,
            help = "Do not remove secrets from output (only applies to non-json output)"
        )]
        include_secrets: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AeroCloudV6Command {
    ListProjects {
        #[arg(short = 's', long)]
        status: Option<ProjectStatus>,
    },
    ListSimulations {
        project_id: Id,

        #[arg(
            short = 'r',
            long,
            default_value = "false",
            help = "displays only completed simulations with their results"
        )]
        show_results: bool,

        #[arg(short = 't', long, help = "filter by status")]
        status: Option<SimulationsV6ListStatus>,

        #[arg(short = 's', long, help = "filter by fluid speed")]
        fluid_speed: Option<FluidSpeed>,

        #[arg(short = 'q', long, help = "filter by quality")]
        quality: Option<SimulationQuality>,

        #[arg(short = 'y', long, help = "filter by yaw angle")]
        yaw_angle: Option<YawAngle>,
    },
    CreateProject {
        name: String,

        #[arg(short = 'd', long)]
        description: Option<String>,
    },
    DeleteProjects {
        #[arg(required = true)]
        project_ids: Vec<Id>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AeroCloudV7Command {
    ListProjects {
        #[arg(short = 's', long)]
        status: Option<ProjectStatus>,
    },

    ListSimulations {
        project_id: Id,

        #[arg(
            short = 'r',
            long,
            default_value = "false",
            help = "displays only completed simulations with their results"
        )]
        show_results: bool,

        #[arg(short = 't', long, help = "filter by status")]
        status: Option<SimulationsV7ListStatus>,

        #[arg(short = 's', long, help = "filter by fluid speed")]
        fluid_speed: Option<FluidSpeed>,

        #[arg(short = 'q', long, help = "filter by quality")]
        quality: Option<SimulationQuality>,

        #[arg(short = 'y', long, help = "filter by yaw angle")]
        yaw_angle: Option<YawAngle>,
    },
    #[command(after_help = format!(r#"
     PARAMS is a JSON file like:

     ```json
     {}```
     "#, include_str!("../examples/aerocloud/v7/create_model.json")))]
    CreateModel {
        #[arg(
            help = "path to file containing params (pass - for reading file from stdin)"
        )]
        params: FileOrStdin,
    },
    CreateProject {
        name: String,

        #[arg(short = 'd', long)]
        description: Option<String>,
    },
    #[command(after_help = format!(r#"
    PARAMS is a JSON file like:

    ```json
    {}```
    "#, include_str!("../examples/aerocloud/v7/create_simulation.json")))]
    CreateSimulation {
        #[arg(short, long)]
        model_id: Option<Id>,

        #[arg(short, long)]
        project_id: Option<Id>,

        #[arg(
            help = "path to file containing params (pass - for reading file from stdin)"
        )]
        params: FileOrStdin,
    },
    #[command(
        about = "Waits for simulations to succeed. If given IDs, will exit after all have succeeded."
    )]
    WaitForSimulations {
        #[arg(required = true, help = "list of IDs to wait for and then exit")]
        ids: Vec<Id>,
    },
    DeleteProjects {
        #[arg(required = true)]
        project_ids: Vec<Id>,
    },
}
