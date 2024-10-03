use crate::config::{Config, Token};
use color_eyre::eyre;
use eyre::WrapErr;
use reqwest::{header, Client, Url};
use std::time::Duration;

static USER_AGENT: &str = concat!("nf-cli", "/", env!("CARGO_PKG_VERSION"),);
static TOKEN_HEADER: &str = "x-nablaflow-token";

pub fn build_aerocloud_client_from_config(
    config: &Config,
) -> eyre::Result<(Client, Url)> {
    let endpoint = config.hostname().join("/aerocloud/graphql")?;

    Ok((build_client(config.token()?)?, endpoint))
}

fn build_client(token: &Token) -> eyre::Result<Client> {
    let mut headers = header::HeaderMap::new();

    let mut token_value = header::HeaderValue::from_str(token)
        .wrap_err("setting token when building http client")?;
    token_value.set_sensitive(true);
    headers.insert(TOKEN_HEADER, token_value);

    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .default_headers(headers)
        .build()
        .wrap_err("building http client")
}
