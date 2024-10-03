use crate::{
    args::{AeroCloudScope, AeroCloudV6Command, Args},
    config::Config,
};
use color_eyre::eyre::{self, WrapErr};

pub mod current_user;
pub mod v6;

pub async fn run(
    args: &Args,
    config: &Config,
    subcommand: &AeroCloudScope,
) -> eyre::Result<()> {
    match subcommand {
        AeroCloudScope::CurrentUser => {
            self::current_user::run(args, config).await
        }
        AeroCloudScope::V6 { command } => match command {
            AeroCloudV6Command::ListProjects => {
                self::v6::list_projects::run(args, config).await
            }
            AeroCloudV6Command::ListSimulations { project_id } => {
                self::v6::list_simulations::run(args, config, project_id).await
            }
            AeroCloudV6Command::CreateModel { params } => {
                self::v6::create_model::run(
                    args,
                    config,
                    &params
                        .clone()
                        .contents()
                        .wrap_err("failed to read contents")?,
                )
                .await
            }
            AeroCloudV6Command::CreateSimulation {
                params,
                project_id,
                model_id,
            } => {
                self::v6::create_simulation::run(
                    args,
                    config,
                    model_id.as_deref(),
                    project_id.as_deref(),
                    &params
                        .clone()
                        .contents()
                        .wrap_err("failed to read contents")?,
                )
                .await
            }
        },
    }
}
