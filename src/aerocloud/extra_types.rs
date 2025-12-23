use crate::aerocloud::types::{
    FileUnit, Fluid, FluidSpeed, GroundOffset, Quaternion, SimulationQuality,
    UpdatePartV7Params, YawAngle, YawAngles,
};
use color_eyre::eyre;
use std::{borrow::Cow, collections::BTreeMap};

#[derive(serde::Deserialize, Clone, Debug)]
pub struct CreateSimulationV7ParamsFromJson {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub revision: Option<String>,

    #[serde(default = "default_quality")]
    pub quality: SimulationQuality,

    #[serde(default = "default_yaw_angles")]
    pub yaw_angles: YawAngles,

    #[serde(default = "default_fluid")]
    pub fluid: Fluid,

    #[serde(default = "default_fluid_speed")]
    pub fluid_speed: FluidSpeed,

    #[serde(default = "default_has_ground")]
    pub has_ground: bool,

    #[serde(default = "default_ground_offset")]
    pub ground_offset: GroundOffset,

    #[serde(default)]
    pub is_ground_moving: bool,
}

impl CreateSimulationV7ParamsFromJson {
    pub fn revision_or_placeholder(&'_ self) -> Cow<'_, str> {
        match &self.revision {
            Some(s) => s.into(),
            None => "<unspecified>".into(),
        }
    }
}

impl Default for CreateSimulationV7ParamsFromJson {
    fn default() -> Self {
        serde_json::from_value(empty_json_dict()).unwrap()
    }
}

const fn default_quality() -> SimulationQuality {
    SimulationQuality::Basic
}

fn default_yaw_angles() -> YawAngles {
    YawAngles(vec![YawAngle(0.0)])
}

const fn default_fluid() -> Fluid {
    Fluid::Air
}

const fn default_fluid_speed() -> FluidSpeed {
    FluidSpeed(10.0)
}

const fn default_has_ground() -> bool {
    true
}

const fn default_ground_offset() -> GroundOffset {
    GroundOffset(0.0)
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FileV7ParamsFromJson {
    #[serde(default = "default_file_unit")]
    pub unit: FileUnit,

    #[serde(default)]
    pub rotation: Option<Quaternion>,

    #[serde(default)]
    pub parts: BTreeMap<String, UpdatePartV7Params>,
}

impl FileV7ParamsFromJson {
    pub fn ensure_is_valid(&self) -> eyre::Result<()> {
        for (name, part) in &self.parts {
            if !part.is_porous.unwrap_or(false) {
                continue;
            }

            if part.darcy_coeff.is_none() {
                eyre::bail!(
                    "part `{name}` is marked as porous but is missing `darcy_coeff`"
                );
            }

            if part.forchheimer_coeff.is_none() {
                eyre::bail!(
                    "part `{name}` is marked as porous but is missing `forchheimer_coeff`"
                );
            }
        }

        Ok(())
    }
}

const fn default_file_unit() -> FileUnit {
    FileUnit::M
}

impl Default for FileV7ParamsFromJson {
    fn default() -> Self {
        serde_json::from_value(empty_json_dict()).unwrap()
    }
}

fn empty_json_dict() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}
