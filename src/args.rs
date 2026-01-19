use crate::{
    aerocloud::{
        NEW_TOKEN_URL,
        types::{
            FluidSpeed, Id, ProjectStatus, SimulationQuality,
            SimulationsV6ListStatus, SimulationsV7ListStatus, YawAngle,
        },
    },
    config::{Config, Token},
};
use clap::{
    Parser, Subcommand,
    builder::styling::{AnsiColor, Styles},
};
use clap_complete::aot::Shell;
use clap_stdin::{FileOrStdin, MaybeStdin};
use reqwest::Url;
use std::{path::PathBuf, time::Duration};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .placeholder(AnsiColor::Blue.on_default());

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = STYLES)]
pub struct Args {
    #[arg(
        short,
        long,
        env = "NF_DEBUG",
        help = "Increase logging for debugging"
    )]
    pub debug: bool,

    #[arg(
        short,
        long,
        env = "NF_LOG_TO_PATH",
        help = "Write logs to the given [PATH] instead of stderr"
    )]
    pub log_to_path: Option<PathBuf>,

    #[arg(
        short,
        long,
        env = "NF_JSON",
        help = "Output in JSON instead of human-friendly tables to allow external tools to integrate"
    )]
    pub json: bool,

    #[arg(
        short = 't',
        long,
        env = "NF_HTTP_TIMEOUT_SECS",
        default_value_t = 360,
        help = "Maximum amount in seconds to wait for HTTP responses to arrive"
    )]
    pub http_timeout_secs: u64,

    #[arg(
        short = 'H',
        long,
        value_name = "URI",
        env = "NF_HOSTNAME",
        help = "Host to connect to. When specified, it will take precedence over the one set in the config"
    )]
    pub hostname: Option<Url>,

    #[arg(
        long,
        env = "NF_AEROCLOUD_AUTH_TOKEN",
        value_name = "AEROCLOUD_AUTH_TOKEN",
        help = "AeroCloud auth token to use during requests. When specified, it will take precedence over the one set in config"
    )]
    pub aerocloud_auth_token: Option<Token>,

    #[arg(
        short,
        long,
        default_value_os_t = Config::default_path().expect("failed to determine default config dir"),
        value_name = "CONFIGPATH",
        env = "NF_CONFIGPATH",
        help = "Configuration file. When specified, it will take precedence over the default on this platform"
    )]
    pub config_path: PathBuf,

    #[arg(
        short,
        long,
        default_value_t = false,
        env = "NF_SKIP_UPDATE_CHECK",
        help = "Skip checking for latest version of this program."
    )]
    pub skip_update_check: bool,

    #[command(subcommand)]
    pub scope: Scope,
}

impl Args {
    pub fn http_timeout(&self) -> Duration {
        Duration::from_secs(self.http_timeout_secs)
    }
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

    #[command(
        about = "Generate completions for major shells (bash, elvish, fish, powershell, zsh)"
    )]
    GenerateCompletions { shell: Shell },

    #[command(about = "Generate man page")]
    GenerateManpage { dest: PathBuf },
}

#[derive(Subcommand, Debug)]
#[command(about = "Actions on AeroCloud")]
pub enum AeroCloudScope {
    #[command(about = "Get information about the current user")]
    CurrentUser,

    #[command(about = "Get information about the token in use")]
    CurrentToken,

    #[command(
        about = format!("Set a new auth token. Generate one here `{}`", NEW_TOKEN_URL)
    )]
    SetAuthToken {
        #[arg(value_name = "TOKEN", help = "Token to set in config")]
        token: MaybeStdin<Token>,
    },

    #[command(about = "Operate on AeroCloud v6 (end-of-life, read-only)")]
    V6 {
        #[command(subcommand)]
        command: AeroCloudV6Command,
    },

    #[command(about = "Operate on AeroCloud v7")]
    V7 {
        #[command(subcommand)]
        command: AeroCloudV7Command,
    },
}

#[derive(Subcommand, Debug)]
#[command(about = "Make changes to the config")]
pub enum ConfigScope {
    #[command(about = "Change the API hostname")]
    SetHostname {
        #[arg(value_name = "HOSTNAME", help = "Hostname to set in config")]
        hostname: Url,
    },

    #[command(about = "Unset a custom API hostname")]
    UnsetHostname,

    #[command(about = "Show the config")]
    Show,
}

#[derive(Subcommand, Debug)]
pub enum AeroCloudV6Command {
    #[command(about = "List projects")]
    ListProjects {
        #[arg(short = 's', long, help = "Filter by project status")]
        status: Option<ProjectStatus>,
    },

    #[command(about = "List simulations")]
    ListSimulations {
        #[arg(help = "A project id from `nf aerocloud v6 list-projects`")]
        project_id: Id,

        #[arg(
            short = 'r',
            long,
            default_value = "false",
            help = "Display only completed simulations with their results"
        )]
        show_results: bool,

        #[arg(short = 't', long, help = "Filter by status")]
        status: Option<SimulationsV6ListStatus>,

        #[arg(short = 's', long, help = "Filter by fluid speed")]
        fluid_speed: Option<FluidSpeed>,

        #[arg(short = 'q', long, help = "Filter by quality")]
        quality: Option<SimulationQuality>,

        #[arg(short = 'y', long, help = "Filter by yaw angle")]
        yaw_angle: Option<YawAngle>,
    },

    #[command(about = "Delete simulations")]
    DeleteSimulations {
        #[arg(
            required = true,
            help = "A list of simulation ids from `nf aerocloud v6 list-simulations`"
        )]
        simulation_ids: Vec<Id>,
    },

    #[command(about = "Create a new project")]
    CreateProject {
        #[arg(help = "Name of the project")]
        name: String,

        #[arg(
            short = 'd',
            long,
            help = "An optional description for the project"
        )]
        description: Option<String>,
    },

    #[command(about = "Delete projects")]
    DeleteProjects {
        #[arg(
            required = true,
            help = "A project id from `nf aerocloud v6 list-projects`"
        )]
        project_ids: Vec<Id>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AeroCloudV7Command {
    #[command(about = "List projects")]
    ListProjects {
        #[arg(short = 's', long, help = "Filter by project status")]
        status: Option<ProjectStatus>,
    },

    #[command(about = "List simulations")]
    ListSimulations {
        #[arg(help = "A project id from `nf aerocloud v7 list-projects`")]
        project_id: Id,

        #[arg(
            short = 'r',
            long,
            default_value = "false",
            help = "Display only completed simulations with their results"
        )]
        show_results: bool,

        #[arg(short = 't', long, help = "Filter by status")]
        status: Option<SimulationsV7ListStatus>,

        #[arg(short = 's', long, help = "Filter by fluid speed")]
        fluid_speed: Option<FluidSpeed>,

        #[arg(short = 'q', long, help = "Filter by quality")]
        quality: Option<SimulationQuality>,

        #[arg(short = 'y', long, help = "Filter by yaw angle")]
        yaw_angle: Option<YawAngle>,
    },

    #[command(about = "Create a new model")]
    CreateModel {
        #[arg(
            help = format!(r#"Path to file containing params (pass - for reading file from stdin).

Example:

```json
{}```
"#, include_str!("../examples/aerocloud/v7/create_model.json"))
        )]
        params: FileOrStdin,
    },

    #[command(about = "Create a new project")]
    CreateProject {
        #[arg(help = "Name of the project")]
        name: String,

        #[arg(
            short = 'd',
            long,
            help = "An optional description for the project"
        )]
        description: Option<String>,
    },

    #[command(about = "Delete simulations")]
    DeleteSimulations {
        #[arg(
            required = true,
            help = "A list of simulation ids from `nf aerocloud v7 list-simulations`"
        )]
        simulation_ids: Vec<Id>,
    },

    #[command(about = "Create a new simulation")]
    CreateSimulation {
        #[arg(
            short,
            long,
            help = "A model id from `nf aerocloud v7 create-model`. When set, it will have precedence over what is read from <PARAMS>"
        )]
        model_id: Option<Id>,

        #[arg(
            short,
            long,
            help = "A project id from `nf aerocloud v7 create-project`. When set, it will have precedence over what is read from <PARAMS>"
        )]
        project_id: Option<Id>,

        #[arg(
            help = format!(r#"Path to file containing params (pass - to read file from stdin).

Example:

```json
{}```
"#, include_str!("../examples/aerocloud/v7/create_simulation.json"))
        )]
        params: FileOrStdin,
    },

    #[command(about = "Wait for one or many simulations to succeed.")]
    WaitForSimulations {
        #[arg(
            required = true,
            help = "List of simulation ids from `nf aerocloud v7 list-simulations`"
        )]
        ids: Vec<Id>,
    },

    #[command(about = "Delete projects")]
    DeleteProjects {
        #[arg(required = true)]
        project_ids: Vec<Id>,
    },

    #[command(
        about = "Start an interactive UI to review and submit multiple simulations at once."
    )]
    Batch {
        #[arg(
            required = false,
            help = "Root dir with simulations, their models and params."
        )]
        root_dir: Option<PathBuf>,
    },
}
