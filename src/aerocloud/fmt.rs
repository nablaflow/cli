use crate::aerocloud::types::{
    BoundaryLayerTreatment, SimulationStatus, Symmetry,
};

pub fn human_simulation_status(v: SimulationStatus) -> &'static str {
    match v {
        SimulationStatus::Progress => "in progress",
        SimulationStatus::Success => "succeeded",
        SimulationStatus::Expired => "expired",
        SimulationStatus::Draft => "draft",
    }
}

pub fn human_boundary_layer_treatment(v: BoundaryLayerTreatment) -> &'static str {
    match v {
        BoundaryLayerTreatment::WallFunctions => "wall functions",
        BoundaryLayerTreatment::ResolvedBoundaryLayer => {
            "resolved boundary layer"
        }
    }
}

pub fn human_symmetry(v: Symmetry) -> &'static str {
    match v {
        Symmetry::Off => "off",
        Symmetry::Center => "center",
        Symmetry::Left => "left",
        Symmetry::Right => "right",
    }
}
