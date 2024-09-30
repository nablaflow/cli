pub mod aerocloud {
    use chrono::NaiveDate;
    use cynic::Id;
    use std::fmt;

    #[cynic::schema("aerocloud")]
    mod schema {}

    #[derive(cynic::Scalar, Debug)]
    pub struct UnsignedInteger(u64);

    impl fmt::Display for UnsignedInteger {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    cynic::impl_scalar!(NaiveDate, schema::Date);

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

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud", graphql_type = "RootQueryType")]
    pub struct ViewerQuery {
        pub viewer: Option<User>,
    }

    #[derive(cynic::QueryFragment, Debug, serde::Serialize)]
    #[cynic(schema = "aerocloud")]
    pub struct ProjectV6 {
        pub id: cynic::Id,
        pub name: String,
        pub browser_url: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "aerocloud", graphql_type = "RootQueryType")]
    pub struct ProjectsV6Query {
        pub projects_v6: Vec<ProjectV6>,
    }
}
