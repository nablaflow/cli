use crate::{
    args::Args,
    config::Config,
    http,
    queries::aerocloud::{
        self, CreateSimulationV6Mutation, CreateSimulationV6MutationParams,
        GroundOffset, InputGroundV6, InputSimulationQuality, InputSimulationV6,
        Speed, YawAngle,
    },
};
use color_eyre::eyre::{self, bail, WrapErr};
use cynic::{http::ReqwestExt, MutationBuilder};
use serde::Deserialize;
use tracing::{debug, error};

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Basic,
    Standard,
    Pro,
}

impl From<Quality> for InputSimulationQuality {
    fn from(val: Quality) -> Self {
        match val {
            Quality::Basic => Self::Basic,
            Quality::Standard => Self::Standard,
            Quality::Pro => Self::Pro,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Fluid {
    Air,
    Water,
}

impl From<Fluid> for aerocloud::Fluid {
    fn from(val: Fluid) -> Self {
        match val {
            Fluid::Air => Self::Air,
            Fluid::Water => Self::Water,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Ground {
    pub enabled: bool,
    pub moving: bool,
    pub offset: f32,
}

impl From<Ground> for InputGroundV6 {
    fn from(val: Ground) -> Self {
        Self {
            enabled: val.enabled,
            moving: val.moving,
            offset: GroundOffset(val.offset),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Simulation {
    pub name: String,
    pub model_id: Option<String>,
    pub project_id: Option<String>,
    pub quality: Quality,
    pub speed: Speed,
    pub fluid: Fluid,
    pub ground: Ground,
    pub yaw_angles: Vec<YawAngle>,
}

impl TryFrom<Simulation> for InputSimulationV6 {
    type Error = eyre::Error;

    fn try_from(val: Simulation) -> Result<Self, Self::Error> {
        Ok(Self {
            name: val.name,
            model_id: val
                .model_id
                .ok_or_else(|| eyre::eyre!("missing model id"))?
                .into(),
            project_id: val
                .project_id
                .ok_or_else(|| eyre::eyre!("missing project id"))?
                .into(),
            fluid: val.fluid.into(),
            ground: val.ground.into(),
            quality: val.quality.into(),
            speed: val.speed,
            yaw_angles: val.yaw_angles,
        })
    }
}

pub async fn run(
    args: &Args,
    config: &Config,
    model_id: Option<&str>,
    project_id: Option<&str>,
    params: &str,
) -> eyre::Result<()> {
    let mut simulation = serde_json::from_str::<Simulation>(params)
        .wrap_err("failed to parse json")?;

    if let Some(id) = model_id {
        simulation.model_id = Some(id.into())
    }
    if let Some(id) = project_id {
        simulation.project_id = Some(id.into())
    }

    let op =
        CreateSimulationV6Mutation::build(CreateSimulationV6MutationParams {
            input: simulation.try_into()?,
        });

    let (client, endpoint) =
        http::build_aerocloud_client(&config.token, &config.hostname)?;

    debug!(
        "{endpoint}, query: {}, variables: {:?}",
        op.query, op.variables
    );

    let res = client
        .post(endpoint)
        .run_graphql(op)
        .await
        .wrap_err("failed to mutate")?;

    if let Some(errors) = res.errors {
        for error in errors {
            error!("{}", error);
        }

        bail!("mutation returned errors");
    }

    let sim = res
        .data
        .ok_or_else(|| eyre::eyre!("bad response"))?
        .create_simulation_v6;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "simulation_id": sim.id,
            }))?
        );
    } else {
        println!("Created simulation with id {}", sim.id.inner());
        println!("Browser url: {}", sim.browser_url);
    }

    Ok(())
}
