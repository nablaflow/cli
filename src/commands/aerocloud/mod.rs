use crate::{
    args::{AeroCloudScope, AeroCloudV6Command, AeroCloudV7Command, Args},
    config::Config,
};
use color_eyre::eyre::{self, WrapErr};

pub mod current_user;
pub mod v6;
pub mod v7;

#[allow(clippy::too_many_lines)]
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
            AeroCloudV6Command::ListProjects {
                status,
                limit,
                page,
            } => {
                self::v6::list_projects::run(args, config, *status, *limit, *page)
                    .await
            }
            AeroCloudV6Command::CreateProject { name, description } => {
                self::v6::create_project::run(
                    args,
                    config,
                    name,
                    description.as_deref(),
                )
                .await
            }
            AeroCloudV6Command::ListSimulations {
                project_id,
                results: true,
                quality,
                speed,
                yaw_angles,
            } => {
                self::v6::list_simulations_results::run(
                    args,
                    config,
                    project_id,
                    *quality,
                    *speed,
                    yaw_angles.as_deref(),
                )
                .await
            }
            AeroCloudV6Command::ListSimulations {
                project_id,
                results: false,
                quality,
                speed,
                ..
            } => {
                self::v6::list_simulations::run(
                    args, config, project_id, *quality, *speed,
                )
                .await
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
        AeroCloudScope::V7 { command } => match command {
            AeroCloudV7Command::ListProjects {
                status,
                limit,
                page,
            } => {
                self::v7::list_projects::run(args, config, *status, *limit, *page)
                    .await
            }
            AeroCloudV7Command::CreateProject { name, description } => {
                self::v7::create_project::run(
                    args,
                    config,
                    name,
                    description.as_deref(),
                )
                .await
            }
            AeroCloudV7Command::ListSimulations {
                project_id,
                results: true,
                quality,
                speed,
                yaw_angles,
            } => {
                self::v7::list_simulations_results::run(
                    args,
                    config,
                    project_id,
                    *quality,
                    *speed,
                    yaw_angles.as_deref(),
                )
                .await
            }
            AeroCloudV7Command::ListSimulations {
                project_id,
                results: false,
                quality,
                speed,
                ..
            } => {
                self::v7::list_simulations::run(
                    args, config, project_id, *quality, *speed,
                )
                .await
            }
            AeroCloudV7Command::CreateModel { params } => {
                self::v7::create_model::run(
                    args,
                    config,
                    &params
                        .clone()
                        .contents()
                        .wrap_err("failed to read contents")?,
                )
                .await
            }
            AeroCloudV7Command::CreateSimulation {
                params,
                project_id,
                model_id,
            } => {
                self::v7::create_simulation::run(
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
