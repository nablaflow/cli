use crate::{
    args::{AeroCloudScope, AeroCloudV6Command, AeroCloudV7Command, Args},
    config::Config,
    http,
};
use color_eyre::eyre::{self, WrapErr};
use std::path::PathBuf;

pub mod current_token;
pub mod current_user;
pub mod set_auth_token;
pub mod v6;
pub mod v7;

#[allow(clippy::too_many_lines)]
pub async fn run(
    args: &Args,
    config: Config,
    subcommand: &AeroCloudScope,
) -> eyre::Result<()> {
    if let AeroCloudScope::SetAuthToken { token } = subcommand {
        return self::set_auth_token::run(args, config, token).await;
    }

    let client =
        http::build_aerocloud_client_from_config(&config, &args.http_timeout())?;

    match subcommand {
        AeroCloudScope::SetAuthToken { .. } => Ok(()),
        AeroCloudScope::CurrentUser => {
            self::current_user::run(args, &client).await
        }
        AeroCloudScope::CurrentToken => {
            self::current_token::run(args, &client).await
        }
        AeroCloudScope::V6 { command } => match command {
            AeroCloudV6Command::ListProjects { status } => {
                self::v6::list_projects::run(args, &client, *status).await
            }
            AeroCloudV6Command::CreateProject { name, description } => {
                self::v6::create_project::run(
                    args,
                    &client,
                    name,
                    description.as_deref(),
                )
                .await
            }
            AeroCloudV6Command::DeleteProjects { project_ids } => {
                self::v6::delete_projects::run(args, &client, project_ids).await
            }
            AeroCloudV6Command::ListSimulations {
                project_id,
                show_results,
                status,
                fluid_speed,
                quality,
                yaw_angle,
            } => {
                self::v6::list_simulations::run(
                    args,
                    &client,
                    project_id,
                    *show_results,
                    *status,
                    *quality,
                    fluid_speed.clone(),
                    yaw_angle.clone(),
                )
                .await
            }
            AeroCloudV6Command::DeleteSimulations { simulation_ids } => {
                self::v6::delete_simulations::run(args, &client, simulation_ids)
                    .await
            }
        },
        AeroCloudScope::V7 { command } => match command {
            AeroCloudV7Command::ListProjects { status } => {
                self::v7::list_projects::run(args, &client, *status).await
            }
            AeroCloudV7Command::CreateProject { name, description } => {
                self::v7::create_project::run(
                    args,
                    &client,
                    name,
                    description.as_deref(),
                )
                .await
            }
            AeroCloudV7Command::DeleteProjects { project_ids } => {
                self::v7::delete_projects::run(args, &client, project_ids).await
            }
            AeroCloudV7Command::ListSimulations {
                project_id,
                show_results,
                status,
                fluid_speed,
                quality,
                yaw_angle,
            } => {
                self::v7::list_simulations::run(
                    args,
                    &client,
                    project_id,
                    *show_results,
                    *status,
                    *quality,
                    fluid_speed.clone(),
                    yaw_angle.clone(),
                )
                .await
            }
            AeroCloudV7Command::CreateModel { params } => {
                self::v7::create_model::run(
                    args,
                    &client,
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
                    &client,
                    model_id.clone(),
                    project_id.clone(),
                    &params
                        .clone()
                        .contents()
                        .wrap_err("failed to read contents")?,
                )
                .await
            }
            AeroCloudV7Command::DeleteSimulations { simulation_ids } => {
                self::v7::delete_simulations::run(args, &client, simulation_ids)
                    .await
            }
            AeroCloudV7Command::WaitForSimulations { ids } => {
                self::v7::wait_for_simulations::run(args, &client, ids).await
            }
            AeroCloudV7Command::Batch { root_dir } => {
                if args.debug && args.log_to_path.is_none() {
                    eyre::bail!(
                        "must log to file, otherwise the UI would get corrupted by logs"
                    );
                }

                self::v7::batch::run(
                    &client,
                    root_dir.as_ref().map(PathBuf::as_path),
                )
                .await
            }
        },
    }
}
