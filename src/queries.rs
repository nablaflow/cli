pub mod aerocloud {
    use chrono::{DateTime, NaiveDate, Utc};
    use cynic::Id;
    use std::fmt;

    #[cynic::schema("aerocloud")]
    mod schema {}

    #[derive(cynic::Scalar, Debug)]
    pub struct UnsignedInteger(pub u64);

    #[derive(cynic::Scalar, Debug)]
    pub struct Speed(pub f32);

    #[derive(cynic::Scalar, Debug)]
    pub struct YawAngle(pub f32);

    #[derive(cynic::Scalar, Debug)]
    pub struct GroundOffset(pub f32);

    impl fmt::Display for UnsignedInteger {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    cynic::impl_scalar!(NaiveDate, schema::Date);
    cynic::impl_scalar!(DateTime<Utc>, schema::DateTime);

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum Plan {
        PayAsYouGo,
        Subscription,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SubscriptionState {
        Active,
        Cancelled,
        Suspended,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SubscriptionInterval {
        Monthly,
        Yearly,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SubscriptionSuspensionReason {
        PaymentRequiresAction,
        PaymentFailed,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SimulationStatus {
        Progress,
        QualityCheck,
        Success,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SimulationQuality {
        Dummy,
        Basic,
        Standard,
        Pro,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum Fluid {
        Air,
        Water,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum FileUnit {
        Mm,
        Cm,
        M,
        Inches,
    }

    #[derive(cynic::Enum, Debug, Clone, Copy, PartialEq, Eq)]
    #[cynic(schema = "aerocloud")]
    pub enum FileUploadStrategy {
        S3,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct User {
        pub id: Id,
        pub email: String,
        pub full_name: Option<String>,
        pub billing: UserBilling,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct UserSubscription {
        pub state: SubscriptionState,
        pub interval: SubscriptionInterval,
        pub monthly_credits: UnsignedInteger,
        pub available_credits: UnsignedInteger,
        pub started_on: NaiveDate,
        pub ends_on: Option<NaiveDate>,
        pub renews_on: Option<NaiveDate>,
        pub next_monthly_cycle_starts_on: Option<NaiveDate>,
        pub suspension_reason: Option<SubscriptionSuspensionReason>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct UserBilling {
        pub current_plan: Plan,
        pub purchased_credits: UnsignedInteger,
        pub purchased_credits_expire_on: Option<NaiveDate>,
        pub total_available_credits: UnsignedInteger,
        pub subscription: Option<UserSubscription>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct ProjectV6 {
        pub id: Id,
        pub name: String,
        pub browser_url: String,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct GroundV6 {
        pub enabled: bool,
        pub moving: bool,
        pub offset: Option<GroundOffset>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud", graphql_type = "SimulationInputsV6")]
    pub struct SimulationInputsV6NoModel {
        pub quality: SimulationQuality,
        pub speed: Speed,
        pub fluid: Fluid,
        pub ground: Option<GroundV6>,
        pub yaw_angles: Vec<YawAngle>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct SimulationV6 {
        pub id: Id,
        pub name: String,
        pub browser_url: String,
        pub status: SimulationStatus,
        pub created_at: DateTime<Utc>,
        pub inputs: SimulationInputsV6NoModel,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud", graphql_type = "ProjectV6")]
    pub struct ProjectV6WithSimulations {
        pub id: Id,
        pub name: String,
        pub simulations: Vec<SimulationV6>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud", graphql_type = "RootQueryType")]
    pub struct ViewerQuery {
        pub viewer: Option<User>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud", graphql_type = "RootQueryType")]
    pub struct ProjectsV6Query {
        pub projects_v6: Vec<ProjectV6>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct SimulationsInProjectV6Arguments {
        pub id: Id,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootQueryType",
        variables = "SimulationsInProjectV6Arguments"
    )]
    pub struct SimulationsInProjectV6Query {
        #[arguments(id: $id)]
        pub project_v6: Option<ProjectV6WithSimulations>,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputFileV6 {
        pub name: String,
        pub unit: FileUnit,
        pub orientation: Option<[f64; 4]>,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputModelV6 {
        pub name: String,
        pub reusable: bool,
        pub files: Vec<InputFileV6>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct FileForUploadV6 {
        pub name: String,
        pub strategy: FileUploadStrategy,
        pub upload_url: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct ModelForUploadV6 {
        pub id: Id,
        pub files: Vec<FileForUploadV6>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct CreateModelV6MutationParams {
        pub input: InputModelV6,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootMutationType",
        variables = "CreateModelV6MutationParams"
    )]
    pub struct CreateModelV6Mutation {
        #[arguments(input: $input)]
        pub create_model_v6: ModelForUploadV6,
    }
}
