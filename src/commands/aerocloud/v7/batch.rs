use crate::aerocloud::{Client, batch};
use color_eyre::eyre;
use std::path::Path;

pub async fn run(client: &Client, path: Option<&Path>) -> eyre::Result<()> {
    batch::run(client, path).await
}
