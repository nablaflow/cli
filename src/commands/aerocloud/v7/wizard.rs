use crate::aerocloud::{Client, wizard};
use color_eyre::eyre;
use std::path::Path;

pub async fn run(client: &Client, path: Option<&Path>) -> eyre::Result<()> {
    wizard::run(client, path).await
}
