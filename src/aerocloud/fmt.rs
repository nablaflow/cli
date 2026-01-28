use crate::aerocloud::types::SimulationStatus;

pub fn human_simulation_status(v: SimulationStatus) -> &'static str {
    match v {
        SimulationStatus::Progress => "in progress",
        SimulationStatus::Success => "succeeded",
        SimulationStatus::Expired => "expired",
        SimulationStatus::Draft => "draft",
    }
}
