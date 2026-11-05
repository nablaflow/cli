use crate::aerocloud::types::{
    BoundaryLayerTreatment, OrganisationApiAccess, SimulationStatus, Symmetry,
};

pub const fn human_simulation_status(v: SimulationStatus) -> &'static str {
    match v {
        SimulationStatus::Progress => "in progress",
        SimulationStatus::Success => "succeeded",
        SimulationStatus::Expired => "expired",
        SimulationStatus::Draft => "draft",
    }
}

pub const fn human_boundary_layer_treatment(
    v: BoundaryLayerTreatment,
) -> &'static str {
    match v {
        BoundaryLayerTreatment::WallFunctions => "wall functions",
        BoundaryLayerTreatment::ResolvedBoundaryLayer => {
            "resolved boundary layer"
        }
    }
}

pub const fn human_symmetry(v: Symmetry) -> &'static str {
    match v {
        Symmetry::Off => "off",
        Symmetry::Center => "center",
        Symmetry::Left => "left",
        Symmetry::Right => "right",
    }
}

pub const fn human_organisation_api_access(
    v: OrganisationApiAccess,
) -> &'static str {
    match v {
        OrganisationApiAccess::None => "none",
        OrganisationApiAccess::Read => "read-only",
        OrganisationApiAccess::ReadWrite => "read/write",
    }
}
