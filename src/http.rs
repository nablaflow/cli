use crate::{
    aerocloud,
    config::{Config, Token},
};
use color_eyre::eyre::{self, WrapErr};
use reqwest::{Client, header};
use std::time::Duration;

static USER_AGENT: &str = concat!("nf-cli", "/", env!("CARGO_PKG_VERSION"),);
static TOKEN_HEADER: &str = "x-nablaflow-token";

pub fn build_aerocloud_client_from_config(
    config: &Config,
) -> eyre::Result<aerocloud::Client> {
    let base_url = config.hostname().join("/aerocloud")?;
    let http_client = build_http_client(config.token_or_fail()?)?;

    Ok(aerocloud::Client::new_with_client(
        base_url.as_ref(),
        http_client,
    ))
}

fn build_http_client(token: &Token) -> eyre::Result<Client> {
    let mut headers = header::HeaderMap::new();

    let mut token_value = header::HeaderValue::from_str(token)
        .wrap_err("setting token when building http client")?;
    token_value.set_sensitive(true);
    headers.insert(TOKEN_HEADER, token_value);

    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .default_headers(headers)
        .build()
        .wrap_err("building http client")
}
