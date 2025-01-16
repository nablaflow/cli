pub mod aerocloud {
    use chrono::{DateTime, NaiveDate, Utc};
    use convert_case::{Case, Casing};
    use cynic::Id;
    use std::fmt;

    #[cynic::schema("aerocloud")]
    mod schema {}

    #[derive(cynic::Scalar, Debug)]
    pub struct UnsignedInteger(pub u64);

    #[derive(cynic::Scalar, Debug)]
    pub struct Area(pub f32);

    impl fmt::Display for Area {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{n:.prec$e} m²", n = self.0, prec = 3)
        }
    }

    #[derive(cynic::Scalar, Debug, Clone, Copy)]
    pub struct Speed(pub f32);

    impl fmt::Display for Speed {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{:.1} m/s", self.0)
        }
    }

    #[derive(cynic::Scalar, Debug)]
    pub struct Force(pub f32);

    impl fmt::Display for Force {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{n:.prec$e} N", n = self.0, prec = 3)
        }
    }

    #[derive(cynic::Scalar, Debug)]
    pub struct Torque(pub f32);

    impl fmt::Display for Torque {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{n:.prec$e} N⋅m", n = self.0, prec = 3)
        }
    }

    #[derive(cynic::Scalar, Debug)]
    pub struct ThermalConductance(pub f32);

    impl fmt::Display for ThermalConductance {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{n:.prec$e} W/K", n = self.0, prec = 3)
        }
    }

    #[derive(cynic::Scalar, Debug)]
    pub struct ThermalTransmittance(pub f32);

    impl fmt::Display for ThermalTransmittance {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{n:.prec$e} W/m²·K", n = self.0, prec = 3)
        }
    }

    #[derive(cynic::Scalar, Debug)]
    pub struct Float(pub f32);

    impl fmt::Display for Float {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{n:.prec$e}", n = self.0, prec = 3)
        }
    }

    #[derive(cynic::Scalar, Debug)]
    pub struct YawAngle(pub f32);

    impl fmt::Display for YawAngle {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(f, "{:.1}°", self.0)
        }
    }

    #[derive(cynic::Scalar, Debug)]
    pub struct GroundOffset(pub f32);

    impl fmt::Display for UnsignedInteger {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(f)
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

    impl fmt::Display for Plan {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", format!("{self:?}").to_case(Case::Lower))
        }
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SubscriptionState {
        Active,
        Cancelled,
        Suspended,
    }

    impl fmt::Display for SubscriptionState {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", format!("{self:?}").to_case(Case::Lower))
        }
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SubscriptionInterval {
        Monthly,
        Yearly,
    }

    impl fmt::Display for SubscriptionInterval {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", format!("{self:?}").to_case(Case::Lower))
        }
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SubscriptionSuspensionReason {
        PaymentRequiresAction,
        PaymentFailed,
    }

    impl fmt::Display for SubscriptionSuspensionReason {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", format!("{self:?}").to_case(Case::Lower))
        }
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum SimulationStatus {
        Progress,
        QualityCheck,
        Success,
    }

    impl fmt::Display for SimulationStatus {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", format!("{self:?}").to_case(Case::Lower))
        }
    }

    #[derive(cynic::Enum, Debug, Clone, Copy, clap::ValueEnum)]
    #[cynic(schema = "aerocloud")]
    pub enum SimulationQuality {
        Dummy,
        Basic,
        Standard,
        Pro,
    }

    impl fmt::Display for SimulationQuality {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", format!("{self:?}").to_case(Case::Lower))
        }
    }

    #[derive(cynic::Enum, Debug, Clone, Copy)]
    #[cynic(schema = "aerocloud")]
    pub enum InputSimulationQuality {
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

    #[derive(cynic::Enum, Debug, Clone, Copy, clap::ValueEnum)]
    #[cynic(schema = "aerocloud")]
    pub enum ProjectStatus {
        Active,
        Closed,
    }

    impl fmt::Display for ProjectStatus {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", format!("{self:?}").to_case(Case::Lower))
        }
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
        pub status: ProjectStatus,
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
    #[cynic(
        schema = "aerocloud",
        graphql_type = "ProjectV6",
        variables = "SimulationsInProjectV6Arguments"
    )]
    pub struct ProjectV6WithSimulations {
        pub id: Id,
        pub name: String,

        #[arguments(quality: $quality, speed: $speed)]
        pub simulations: Vec<SimulationV6>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud", graphql_type = "RootQueryType")]
    pub struct ViewerQuery {
        pub viewer: Option<User>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootQueryType",
        variables = "ProjectsV6Arguments"
    )]
    pub struct ProjectsV6Query {
        #[arguments(status: $status, limit: $limit, offset: $offset)]
        pub projects_v6: Vec<ProjectV6>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct ProjectsV6Arguments {
        pub limit: UnsignedInteger,
        pub offset: UnsignedInteger,
        pub status: Option<ProjectStatus>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct SimulationsInProjectV6Arguments {
        pub id: Id,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub quality: Option<SimulationQuality>,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub speed: Option<Speed>,
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

    #[derive(cynic::QueryVariables, Debug)]
    pub struct SimulationsWithResultsInProjectV6Arguments {
        pub id: Id,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub quality: Option<SimulationQuality>,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub speed: Option<Speed>,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub yaw_angles: Option<Vec<YawAngle>>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootQueryType",
        variables = "SimulationsWithResultsInProjectV6Arguments"
    )]
    pub struct SimulationsWithResultsInProjectV6Query {
        #[arguments(id: $id)]
        pub project_v6: Option<ProjectV6WithSimulationsResults>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "ProjectV6",
        variables = "SimulationsWithResultsInProjectV6Arguments"
    )]
    pub struct ProjectV6WithSimulationsResults {
        pub id: Id,
        pub name: String,
        #[arguments(quality: $quality, speed: $speed)]
        pub simulations: Vec<SimulationV6WithResults>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "SimulationV6",
        variables = "SimulationsWithResultsInProjectV6Arguments"
    )]
    pub struct SimulationV6WithResults {
        pub id: Id,
        pub name: String,
        pub browser_url: String,
        pub status: SimulationStatus,
        pub created_at: DateTime<Utc>,
        pub inputs: SimulationInputsV6NoModel,
        pub results: Option<ResultsV6>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(
        schema = "aerocloud",
        variables = "SimulationsWithResultsInProjectV6Arguments"
    )]
    pub struct ResultsV6 {
        #[arguments(yawAngles: $yaw_angles)]
        pub yaw_angles: Option<Vec<YawAngleResultsV6>>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleResultsV6 {
        pub degrees: YawAngle,
        pub surface: Option<Area>,
        pub force: YawAngleForceV6,
        pub coefficient: YawAngleCoefficientV6,
        pub coefficient_area: YawAngleCoefficientAreaV6,
        pub moment: YawAngleMomentV6,
        pub heat_transfer: HeatTransferV6,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleForceV6 {
        pub fd: Force,
        pub fl: Force,
        pub fs: Force,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleCoefficientV6 {
        pub cd: Float,
        pub cl: Float,
        pub cs: Float,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleCoefficientAreaV6 {
        pub cda: Area,
        pub cla: Area,
        pub csa: Area,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleMomentV6 {
        pub mr: Torque,
        pub mp: Torque,
        pub my: Torque,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct HeatTransferV6 {
        pub value: Option<ThermalConductance>,
        pub coefficient: Option<ThermalTransmittance>,
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

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputGroundV6 {
        pub enabled: bool,
        pub moving: bool,
        pub offset: GroundOffset,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputSimulationV6 {
        pub name: String,
        pub model_id: Id,
        pub project_id: Id,
        pub quality: InputSimulationQuality,
        pub speed: Speed,
        pub fluid: Fluid,
        pub ground: InputGroundV6,
        pub yaw_angles: Vec<YawAngle>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct CreateSimulationV6MutationParams {
        pub input: InputSimulationV6,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootMutationType",
        variables = "CreateSimulationV6MutationParams"
    )]
    pub struct CreateSimulationV6Mutation {
        #[arguments(input: $input)]
        pub create_simulation_v6: SimulationV6,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct CreateProjectV6MutationParams {
        pub input: InputProjectV6,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootMutationType",
        variables = "CreateProjectV6MutationParams"
    )]
    pub struct CreateProjectV6Mutation {
        #[arguments(input: $input)]
        pub create_project_v6: ProjectV6,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputProjectV6 {
        pub name: String,
        pub description: Option<String>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct ProjectV7 {
        pub id: Id,
        pub name: String,
        pub status: ProjectStatus,
        pub browser_url: String,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct GroundV7 {
        pub enabled: bool,
        pub moving: bool,
        pub offset: Option<GroundOffset>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud", graphql_type = "SimulationInputsV7")]
    pub struct SimulationInputsV7NoModel {
        pub quality: SimulationQuality,
        pub speed: Speed,
        pub fluid: Fluid,
        pub ground: Option<GroundV7>,
        pub yaw_angles: Vec<YawAngle>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct SimulationV7 {
        pub id: Id,
        pub name: String,
        pub browser_url: String,
        pub status: SimulationStatus,
        pub created_at: DateTime<Utc>,
        pub inputs: SimulationInputsV7NoModel,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "ProjectV7",
        variables = "SimulationsInProjectV7Arguments"
    )]
    pub struct ProjectV7WithSimulations {
        pub id: Id,
        pub name: String,

        #[arguments(quality: $quality, speed: $speed)]
        pub simulations: Vec<SimulationV7>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootQueryType",
        variables = "ProjectsV7Arguments"
    )]
    pub struct ProjectsV7Query {
        #[arguments(status: $status, limit: $limit, offset: $offset)]
        pub projects_v7: Vec<ProjectV7>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct ProjectsV7Arguments {
        pub limit: UnsignedInteger,
        pub offset: UnsignedInteger,
        pub status: Option<ProjectStatus>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct SimulationsInProjectV7Arguments {
        pub id: Id,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub quality: Option<SimulationQuality>,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub speed: Option<Speed>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootQueryType",
        variables = "SimulationsInProjectV7Arguments"
    )]
    pub struct SimulationsInProjectV7Query {
        #[arguments(id: $id)]
        pub project_v7: Option<ProjectV7WithSimulations>,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputFileV7 {
        pub name: String,
        pub unit: FileUnit,
        pub orientation: Option<[f64; 4]>,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputModelV7 {
        pub name: String,
        pub reusable: bool,
        pub files: Vec<InputFileV7>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct FileForUploadV7 {
        pub name: String,
        pub strategy: FileUploadStrategy,
        pub upload_url: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct ModelForUploadV7 {
        pub id: Id,
        pub files: Vec<FileForUploadV7>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct CreateModelV7MutationParams {
        pub input: InputModelV7,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootMutationType",
        variables = "CreateModelV7MutationParams"
    )]
    pub struct CreateModelV7Mutation {
        #[arguments(input: $input)]
        pub create_model_v7: ModelForUploadV7,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputGroundV7 {
        pub enabled: bool,
        pub moving: bool,
        pub offset: GroundOffset,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputSimulationV7 {
        pub name: String,
        pub model_id: Id,
        pub project_id: Id,
        pub quality: InputSimulationQuality,
        pub speed: Speed,
        pub fluid: Fluid,
        pub ground: InputGroundV7,
        pub yaw_angles: Vec<YawAngle>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct CreateSimulationV7MutationParams {
        pub input: InputSimulationV7,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootMutationType",
        variables = "CreateSimulationV7MutationParams"
    )]
    pub struct CreateSimulationV7Mutation {
        #[arguments(input: $input)]
        pub create_simulation_v7: SimulationV7,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct CreateProjectV7MutationParams {
        pub input: InputProjectV7,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootMutationType",
        variables = "CreateProjectV7MutationParams"
    )]
    pub struct CreateProjectV7Mutation {
        #[arguments(input: $input)]
        pub create_project_v7: ProjectV7,
    }

    #[derive(cynic::InputObject, Debug)]
    #[cynic(schema = "aerocloud")]
    pub struct InputProjectV7 {
        pub name: String,
        pub description: Option<String>,
    }

    #[derive(cynic::QueryVariables, Debug)]
    pub struct SimulationsWithResultsInProjectV7Arguments {
        pub id: Id,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub quality: Option<SimulationQuality>,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub speed: Option<Speed>,

        #[cynic(skip_serializing_if = "Option::is_none")]
        pub yaw_angles: Option<Vec<YawAngle>>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "RootQueryType",
        variables = "SimulationsWithResultsInProjectV7Arguments"
    )]
    pub struct SimulationsWithResultsInProjectV7Query {
        #[arguments(id: $id)]
        pub project_v7: Option<ProjectV7WithSimulationsResults>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "ProjectV7",
        variables = "SimulationsWithResultsInProjectV7Arguments"
    )]
    pub struct ProjectV7WithSimulationsResults {
        pub id: Id,
        pub name: String,
        #[arguments(quality: $quality, speed: $speed)]
        pub simulations: Vec<SimulationV7WithResults>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(
        schema = "aerocloud",
        graphql_type = "SimulationV7",
        variables = "SimulationsWithResultsInProjectV7Arguments"
    )]
    pub struct SimulationV7WithResults {
        pub id: Id,
        pub name: String,
        pub browser_url: String,
        pub status: SimulationStatus,
        pub created_at: DateTime<Utc>,
        pub inputs: SimulationInputsV7NoModel,
        pub results: Option<ResultsV7>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(
        schema = "aerocloud",
        variables = "SimulationsWithResultsInProjectV7Arguments"
    )]
    pub struct ResultsV7 {
        #[arguments(yawAngles: $yaw_angles)]
        pub yaw_angles: Option<Vec<YawAngleResultsV7>>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleResultsV7 {
        pub degrees: YawAngle,
        pub surface: Area,
        pub force: YawAngleForceV7,
        pub coefficient: YawAngleCoefficientV7,
        pub coefficient_area: YawAngleCoefficientAreaV7,
        pub moment: YawAngleMomentV7,
        pub heat_transfer: HeatTransferV7,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleForceV7 {
        pub fd: Force,
        pub fl: Force,
        pub fs: Force,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleCoefficientV7 {
        pub cd: Float,
        pub cl: Float,
        pub cs: Float,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleCoefficientAreaV7 {
        pub cda: Area,
        pub cla: Area,
        pub csa: Area,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct YawAngleMomentV7 {
        pub mr: Torque,
        pub mp: Torque,
        pub my: Torque,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct HeatTransferV7 {
        pub value: ThermalConductance,
        pub coefficient: ThermalTransmittance,
    }
}
