#![allow(
    clippy::default_trait_access,
    clippy::doc_markdown,
    clippy::unused_self,
    clippy::match_same_arms,
    dead_code
)]

use crate::aerocloud::types::{IdempotencyKey, JsonErrorResponse};
use color_eyre::eyre::Report;
use uuid::Uuid;

pub fn new_idempotency_key() -> IdempotencyKey {
    IdempotencyKey(Uuid::new_v4().to_string())
}

pub fn fmt_progenitor_err(err: Error<JsonErrorResponse>) -> Report {
    let Error::ErrorResponse(res) = err else {
        return err.into();
    };

    let mut table = comfy_table::Table::new();
    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec!["Attribute", "Reason"]);

    for error in &res.errors {
        table.add_row(vec![&error.source.pointer, &error.detail]);
    }

    Report::msg(table.to_string())
}

include!(concat!(env!("OUT_DIR"), "/codegen_aerocloud.rs"));
