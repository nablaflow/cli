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

/// An API error with a human-friendly message.
///
/// Prefer it over [`Error`], whose message dumps the whole response, headers included.
#[derive(Debug)]
pub struct ApiError(Box<Error<JsonErrorResponse>>);

impl From<Error<JsonErrorResponse>> for ApiError {
    fn from(err: Error<JsonErrorResponse>) -> Self {
        Self(Box::new(err))
    }
}

impl ApiError {
    pub fn status(&self) -> Option<StatusCode> {
        self.0.status()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Error::ErrorResponse(ref res) = *self.0 else {
            return self.0.fmt(f);
        };

        let status = res.status();
        let errors = &res.errors;

        // Errors that do not point to an attribute are about the whole request.
        if errors
            .iter()
            .all(|e| matches!(e.source.pointer.as_str(), "" | "/"))
        {
            let details = errors
                .iter()
                .map(|e| e.detail.as_str())
                .collect::<Vec<_>>()
                .join("; ");

            return if details.is_empty() {
                write!(f, "API responded with {status}")
            } else {
                write!(f, "API responded with {status}: {details}")
            };
        }

        let mut table = new_dynamic_table();
        table.set_header(vec!["Attribute", "Reason"]);

        for error in errors {
            table.add_row(vec![&error.source.pointer, &error.detail]);
        }

        write!(f, "API responded with {status}:\n{table}")
    }
}

impl std::error::Error for ApiError {
    // Skips the wrapped error itself, its message would repeat this one.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

pub fn fmt_progenitor_err(err: Error<JsonErrorResponse>) -> Report {
    ApiError::from(err).into()
}

/// Decides whether a failed request can be retried.
///
/// POSTs that can fail with 409 carry an idempotency key, so retrying them is safe:
/// 409 means the same request is still being processed, 5xx responses are not recorded.
/// The remaining requests are either reads or idempotent updates.
pub fn retry_failure(err: impl Into<ApiError>) -> Failure {
    let err = err.into();
    let status = err.status();
    let is_transport_err = matches!(
        *err.0,
        Error::CommunicationError(..) | Error::ResponseBodyError(..)
    );
    let report = Report::new(err);

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

#[cfg(test)]
mod tests {
    use super::{
        ApiError, Error, ResponseValue,
        types::{
            JsonErrorResponse, JsonErrorResponseErrorsItem,
            JsonErrorResponseErrorsItemSource,
        },
    };
    use reqwest::{StatusCode, header::HeaderMap};

    fn api_error(status: StatusCode, errors: &[(&str, &str)]) -> ApiError {
        let errors = errors
            .iter()
            .map(|(pointer, detail)| JsonErrorResponseErrorsItem {
                detail: (*detail).into(),
                source: JsonErrorResponseErrorsItemSource {
                    pointer: (*pointer).into(),
                },
                title: "title".into(),
            })
            .collect();

        Error::ErrorResponse(ResponseValue::new(
            JsonErrorResponse { errors },
            status,
            HeaderMap::new(),
        ))
        .into()
    }

    #[test]
    fn request_wide_errors_are_inlined() {
        let err = api_error(
            StatusCode::FORBIDDEN,
            &[("/", "you have no API access to this organisation")],
        );

        assert_eq!(
            err.to_string(),
            "API responded with 403 Forbidden: you have no API access to this organisation"
        );
    }

    #[test]
    fn errors_without_details_show_status() {
        let err = api_error(StatusCode::FORBIDDEN, &[]);

        assert_eq!(err.to_string(), "API responded with 403 Forbidden");
    }

    #[test]
    fn attribute_errors_are_tabulated() {
        let msg = api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            &[("/name", "can't be blank")],
        )
        .to_string();

        assert!(
            msg.starts_with("API responded with 422 Unprocessable Entity:\n")
        );
        assert!(msg.contains("/name"));
        assert!(msg.contains("can't be blank"));
    }

    #[test]
    fn non_response_errors_keep_their_message() {
        let err = ApiError::from(Error::InvalidRequest("nope".into()));

        assert_eq!(err.to_string(), "Invalid Request: nope");
    }
}
