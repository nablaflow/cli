use crate::{
    aerocloud,
    args::Args,
    config::{Config, Token},
};
use color_eyre::eyre::{self, WrapErr};
use reqwest::{Client, header};
use std::time::Duration;
use tower::limit::concurrency::ConcurrencyLimitLayer;

static USER_AGENT: &str = concat!("nf-cli", "/", env!("CARGO_PKG_VERSION"),);
static TOKEN_HEADER: &str = "x-nablaflow-token";

pub fn build_aerocloud_client(
    config: &Config,
    args: &Args,
) -> eyre::Result<aerocloud::Client> {
    let base_url = config.hostname().join("/aerocloud")?;

    let http_client = build_http_client(
        Some(config.aerocloud_token_or_fail()?),
        &args.api_http_timeout(),
        args.api_request_concurrency,
    )?;

    Ok(aerocloud::Client::new_with_client(
        base_url.as_ref(),
        http_client,
    ))
}

pub fn build_file_upload_client(args: &Args) -> eyre::Result<Client> {
    build_http_client(
        None,
        &args.file_upload_http_timeout(),
        args.file_upload_concurrency,
    )
}

fn build_http_client(
    token: Option<&Token>,
    timeout: &Duration,
    concurrency_limit: usize,
) -> eyre::Result<Client> {
    let mut headers = header::HeaderMap::new();

    if let Some(token) = token {
        let mut token_value = header::HeaderValue::from_str(token)
            .wrap_err("setting token when building http client")?;
        token_value.set_sensitive(true);
        headers.insert(TOKEN_HEADER, token_value);
    }

    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(*timeout)
        .default_headers(headers)
        .connector_layer(ConcurrencyLimitLayer::new(concurrency_limit))
        .build()
        .wrap_err("building http client")
}
