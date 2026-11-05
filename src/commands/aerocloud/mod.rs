use crate::{
    aerocloud::{Client, Workspace, v7::V7Client},
    args::{
        AeroCloudOrgCommand, AeroCloudScope, AeroCloudV6Command,
        AeroCloudV7Command, Args,
    },
    commands::aerocloud::v7::{
        batch::WorkspaceLabel, model_submission::ModelSubmitter,
    },
    config::Config,
    http,
};
use color_eyre::eyre::{self, WrapErr};
use std::path::PathBuf;

pub mod current_token;
pub mod current_user;
pub mod org;
pub mod set_auth_token;
pub mod set_org;
pub mod unset_org;
pub mod v6;
pub mod v7;

#[allow(clippy::too_many_lines)]
pub async fn run(
    args: &Args,
    config: Config,
    subcommand: &AeroCloudScope,
) -> eyre::Result<()> {
    match subcommand {
        AeroCloudScope::SetAuthToken { token } => {
            return self::set_auth_token::run(args, config, token).await;
        }
        AeroCloudScope::UnsetOrg => {
            return self::unset_org::run(args).await;
        }
        _ => {}
    }

    let api_client = http::build_aerocloud_client(&config, args)
        .context("building api client")?;

    match subcommand {
        AeroCloudScope::SetAuthToken { .. } | AeroCloudScope::UnsetOrg => Ok(()),
        AeroCloudScope::SetOrg { org_id } => {
            self::set_org::run(args, &api_client, org_id).await
        }
        AeroCloudScope::CurrentUser => {
            self::current_user::run(args, &api_client).await
        }
        AeroCloudScope::CurrentToken => {
            self::current_token::run(args, &api_client).await
        }
        AeroCloudScope::Org { command } => match command {
            AeroCloudOrgCommand::List { members } => {
                self::org::list::run(args, &api_client, *members).await
            }
        },
        AeroCloudScope::V6 { command } => match command {
            AeroCloudV6Command::ListProjects { status } => {
                self::v6::list_projects::run(args, &api_client, *status).await
            }
            AeroCloudV6Command::CreateProject { name, description } => {
                self::v6::create_project::run(
                    args,
                    &api_client,
                    name,
                    description.as_deref(),
                )
                .await
            }
            AeroCloudV6Command::DeleteProjects { project_ids } => {
                self::v6::delete_projects::run(args, &api_client, project_ids)
                    .await
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
                    &api_client,
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
                self::v6::delete_simulations::run(
                    args,
                    &api_client,
                    simulation_ids,
                )
                .await
            }
        },
        AeroCloudScope::V7 {
            org,
            personal,
            command,
        } => {
            let workspace = Workspace::pick(
                *personal,
                org.clone(),
                config.aerocloud_org.as_ref().map(|org| org.id.clone()),
            );

            run_v7(
                args,
                &config,
                &api_client,
                V7Client::new(api_client.clone(), workspace),
                command,
            )
            .await
        }
    }
}

async fn run_v7(
    args: &Args,
    config: &Config,
    client: &Client,
    api_client: V7Client,
    command: &AeroCloudV7Command,
) -> eyre::Result<()> {
    match command {
        AeroCloudV7Command::ListProjects { status } => {
            self::v7::list_projects::run(args, &api_client, *status).await
        }
        AeroCloudV7Command::CreateProject { name, description } => {
            self::v7::create_project::run(
                args,
                &api_client,
                name,
                description.as_deref(),
            )
            .await
        }
        AeroCloudV7Command::DeleteProjects { project_ids } => {
            self::v7::delete_projects::run(args, &api_client, project_ids).await
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
                &api_client,
                project_id,
                *show_results,
                *status,
                *quality,
                fluid_speed.clone(),
                yaw_angle.clone(),
            )
            .await
        }
        AeroCloudV7Command::ListReusableModels => {
            self::v7::list_reusable_models::run(args, &api_client).await
        }
        AeroCloudV7Command::CreateModel { params } => {
            self::v7::create_model::run(
                args,
                &ModelSubmitter::new(
                    args,
                    config,
                    api_client.workspace().clone(),
                )?,
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
                &api_client,
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
            self::v7::delete_simulations::run(args, &api_client, simulation_ids)
                .await
        }
        AeroCloudV7Command::WaitForSimulations { ids } => {
            self::v7::wait_for_simulations::run(args, &api_client, ids).await
        }
        AeroCloudV7Command::Batch { root_dir } => {
            self::v7::batch::run(
                &api_client,
                WorkspaceLabel::resolve(client, api_client.workspace()).await,
                &ModelSubmitter::new(
                    args,
                    config,
                    api_client.workspace().clone(),
                )?,
                root_dir.as_ref().map(PathBuf::as_path),
            )
            .await
        }
    }
}
