#![allow(
    clippy::default_trait_access,
    clippy::doc_markdown,
    clippy::unused_self,
    dead_code
)]

use crate::aerocloud::types::IdempotencyKey;
use uuid::Uuid;

pub fn new_idempotency_key() -> IdempotencyKey {
    IdempotencyKey(Uuid::new_v4().to_string())
}

include!(concat!(env!("OUT_DIR"), "/codegen_aerocloud.rs"));
