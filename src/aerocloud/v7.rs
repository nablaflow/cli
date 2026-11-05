//! AeroCloud v7 client that targets either personal or organisation endpoints.
//!
//! Every method calls the personal endpoint (e.g. `/v7/projects`) when no organisation is
//! set, or its organisation counterpart (e.g. `/org/{org_id}/v7/projects`) otherwise.
//! Both are generated from the schema, so any divergence between the two breaks the build.

use super::{
    ApiError, ByteStream, Client, Error, ResponseValue,
    types::{
        CreateModelV7Params, CreateProjectV7Params, CreateSimulationV7Params,
        FluidSpeed, Id, IdempotencyKey, ListPageModelsV7, ListPageProjectsV7,
        ListPageSimulationsV7, ModelV7, OrgSimulationsV7ListStatus,
        PaginationLimit, PaginationOffset, ProjectStatus, ProjectV7,
        SimulationQuality, SimulationV7, SimulationsV7ListStatus,
        UpdatePartV7Params, YawAngle,
    },
};

type Res<T> = Result<ResponseValue<T>, ApiError>;

#[derive(Debug, Clone)]
pub struct V7Client {
    inner: Client,
    org_id: Option<Id>,
}

/// Generates methods that forward their arguments as-is to either endpoint.
macro_rules! dispatch {
    ($(
        fn $name:ident / $org_name:ident ( $($arg:ident : $ty:ty),* $(,)? ) -> $ret:ty;
    )*) => {
        $(
            pub async fn $name(&self, $($arg: $ty),*) -> Res<$ret> {
                match &self.org_id {
                    None => self.inner.$name($($arg),*).await,
                    Some(org_id) => self.inner.$org_name(org_id, $($arg),*).await,
                }
                .map_err(ApiError::from)
            }
        )*
    };
}

impl V7Client {
    pub const fn new(inner: Client, org_id: Option<Id>) -> Self {
        Self { inner, org_id }
    }

    pub const fn org_id(&self) -> Option<&Id> {
        self.org_id.as_ref()
    }

    dispatch! {
        fn projects_v7_list / org_projects_v7_list(
            limit: Option<&PaginationLimit>,
            offset: Option<&PaginationOffset>,
            status: Option<ProjectStatus>,
        ) -> ListPageProjectsV7;

        fn projects_v7_get / org_projects_v7_get(id: &Id) -> ProjectV7;

        fn projects_v7_create / org_projects_v7_create(
            idempotency_key: &IdempotencyKey,
            body: &CreateProjectV7Params,
        ) -> ProjectV7;

        fn projects_v7_delete / org_projects_v7_delete(id: &Id) -> ByteStream;

        fn simulations_v7_get / org_simulations_v7_get(id: &Id) -> SimulationV7;

        fn simulations_v7_create / org_simulations_v7_create(
            idempotency_key: &IdempotencyKey,
            body: &CreateSimulationV7Params,
        ) -> SimulationV7;

        fn simulations_v7_delete / org_simulations_v7_delete(id: &Id) -> ByteStream;

        fn models_v7_create / org_models_v7_create(
            idempotency_key: &IdempotencyKey,
            body: &CreateModelV7Params,
        ) -> ModelV7;

        fn models_v7_get / org_models_v7_get(id: &Id) -> ModelV7;

        fn models_v7_finalise / org_models_v7_finalise(
            id: &Id,
            idempotency_key: &IdempotencyKey,
        ) -> ModelV7;

        fn parts_v7_update / org_parts_v7_update(
            model_id: &Id,
            id: &Id,
            body: &UpdatePartV7Params,
        ) -> ByteStream;
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn simulations_v7_list(
        &self,
        project_id: &Id,
        fluid_speed: Option<&FluidSpeed>,
        limit: Option<&PaginationLimit>,
        offset: Option<&PaginationOffset>,
        quality: Option<SimulationQuality>,
        status: Option<SimulationsV7ListStatus>,
        yaw_angle: Option<&YawAngle>,
    ) -> Res<ListPageSimulationsV7> {
        match &self.org_id {
            None => {
                self.inner
                    .simulations_v7_list(
                        project_id,
                        fluid_speed,
                        limit,
                        offset,
                        quality,
                        status,
                        yaw_angle,
                    )
                    .await
            }
            Some(org_id) => {
                self.inner
                    .org_simulations_v7_list(
                        org_id,
                        project_id,
                        fluid_speed,
                        limit,
                        offset,
                        quality,
                        status.map(org_simulations_list_status),
                        yaw_angle,
                    )
                    .await
            }
        }
        .map_err(ApiError::from)
    }

    pub async fn models_v7_list_reusable(
        &self,
        limit: Option<&PaginationLimit>,
        offset: Option<&PaginationOffset>,
    ) -> Res<ListPageModelsV7> {
        if self.org_id.is_some() {
            return Err(Error::InvalidRequest(
                "reusable models are not available for organisations".into(),
            )
            .into());
        }

        Ok(self.inner.models_v7_list_reusable(limit, offset).await?)
    }
}

const fn org_simulations_list_status(
    v: SimulationsV7ListStatus,
) -> OrgSimulationsV7ListStatus {
    match v {
        SimulationsV7ListStatus::InProgress => {
            OrgSimulationsV7ListStatus::InProgress
        }
        SimulationsV7ListStatus::Completed => {
            OrgSimulationsV7ListStatus::Completed
        }
        SimulationsV7ListStatus::Draft => OrgSimulationsV7ListStatus::Draft,
    }
}
