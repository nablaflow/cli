#![allow(
    clippy::default_trait_access,
    clippy::doc_markdown,
    clippy::match_same_arms,
    clippy::missing_const_for_fn,
    clippy::struct_excessive_bools,
    clippy::unnecessary_trailing_comma,
    clippy::unused_self,
    clippy::use_self,
    dead_code
)]

use crate::{
    aerocloud::types::{IdempotencyKey, JsonErrorResponse},
    retry::{self, Failure},
    utils::new_dynamic_table,
};
use color_eyre::eyre::Report;
use reqwest::StatusCode;
use uuid::Uuid;

pub mod extra_types;
pub mod fmt;

pub const NEW_TOKEN_URL: &str = "https://aerocloud.nablaflow.io/developer/api";

pub fn new_idempotency_key() -> IdempotencyKey {
    IdempotencyKey(Uuid::new_v4().to_string())
}

pub fn fmt_progenitor_err(err: Error<JsonErrorResponse>) -> Report {
    let Error::ErrorResponse(res) = err else {
        return err.into();
    };

    let mut table = new_dynamic_table();
    table.set_header(vec!["Attribute", "Reason"]);

    for error in &res.errors {
        table.add_row(vec![&error.source.pointer, &error.detail]);
    }

    Report::msg(format!("Error in API response:\n{table}"))
}

/// Decides whether a failed request can be retried.
///
/// POSTs that can fail with 409 carry an idempotency key, so retrying them is safe:
/// 409 means the same request is still being processed, 5xx responses are not recorded.
/// The remaining requests are either reads or idempotent updates.
pub fn retry_failure(err: Error<JsonErrorResponse>) -> Failure {
    let status = err.status();
    let is_transport_err = matches!(
        err,
        Error::CommunicationError(..) | Error::ResponseBodyError(..)
    );
    let report = fmt_progenitor_err(err);

    match status {
        Some(StatusCode::CONFLICT) => Failure::InProgress(report),
        Some(status) if retry::is_transient_status(status) => {
            Failure::Transient(report)
        }
        None if is_transport_err => Failure::Transient(report),
        Some(_) | None => Failure::Fatal(report),
    }
}

include!(concat!(env!("OUT_DIR"), "/codegen_aerocloud.rs"));
