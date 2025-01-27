use crate::config::{Config, Token};
use async_tungstenite::tungstenite::{
    client::IntoClientRequest, http::HeaderValue,
};
use color_eyre::eyre::{self, WrapErr};
use cynic::GraphQlError;
use graphql_ws_client::Client as WsClient;
use itertools::Itertools;
use reqwest::{header, Client, Url};
use std::{borrow::Cow, future::IntoFuture, time::Duration};

static USER_AGENT: &str = concat!("nf-cli", "/", env!("CARGO_PKG_VERSION"),);
static TOKEN_HEADER: &str = "x-nablaflow-token";

pub fn build_aerocloud_client_from_config(
    config: &Config,
) -> eyre::Result<(Client, Url)> {
    let endpoint = config.hostname().join("/aerocloud/graphql")?;

    Ok((build_client(config.token_or_fail()?)?, endpoint))
}

pub async fn build_aerocloud_ws_client_from_config(
    config: &Config,
) -> eyre::Result<(WsClient, Url)> {
    let endpoint = config.ws_hostname().join("/aerocloud/subscriptions")?;
    let client = build_ws_client(&endpoint, config.token_or_fail()?).await?;

    Ok((client, endpoint))
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

async fn build_ws_client(
    endpoint: &Url,
    token: &Token,
) -> eyre::Result<WsClient> {
    let mut request = endpoint
        .into_client_request()
        .wrap_err("building ws request")?;

    let mut token_value = header::HeaderValue::from_str(token)
        .wrap_err("setting token when building ws client")?;
    token_value.set_sensitive(true);

    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws").unwrap(),
    );
    request.headers_mut().insert(TOKEN_HEADER, token_value);
    request
        .headers_mut()
        .insert("user-agent", HeaderValue::from_str(USER_AGENT).unwrap());

    let (connection, _) = async_tungstenite::tokio::connect_async(request)
        .await
        .wrap_err("connecting to ws endpoint")?;

    let (client, actor) = WsClient::build(connection)
        .await
        .wrap_err("passing the ws connection to graphql client")?;
    tokio::spawn(actor.into_future());

    Ok(client)
}

pub fn format_graphql_errors<ErrorExtensions>(
    errors: Option<Vec<GraphQlError<ErrorExtensions>>>,
) -> Cow<'static, str> {
    let Some(errors) = errors else {
        return "<unknown>".into();
    };

    errors
        .into_iter()
        .map(|error| error.message)
        .join("\n")
        .into()
}
