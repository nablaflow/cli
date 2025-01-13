use crate::{
    args::{Args, ConfigScope},
    config::Config,
};
use color_eyre::eyre;

pub mod set_auth_token;
pub mod set_hostname;
pub mod show;
pub mod unset_hostname;

pub async fn run(
    args: &Args,
    config: &Config,
    subcommand: &ConfigScope,
) -> eyre::Result<()> {
    match subcommand {
        ConfigScope::SetAuthToken { token } => {
            self::set_auth_token::run(args, config, token).await
        }
        ConfigScope::SetHostname { hostname } => {
            self::set_hostname::run(args, config, hostname).await
        }
        ConfigScope::UnsetHostname => {
            self::unset_hostname::run(args, config).await
        }
        ConfigScope::Show { include_secrets } => {
            self::show::run(args, config, *include_secrets)
        }
    }
}
