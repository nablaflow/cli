use crate::args::Args;
use color_eyre::eyre::{self, WrapErr};
use reqwest::Url;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::{Path, PathBuf};
use tokio::fs as tokio_fs;

pub const DEFAULT_HOSTNAME: &str = "https://api.nablaflow.io";

pub type Token = String;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aerocloud_token: Option<String>,

    #[serde(
        serialize_with = "serialize_url",
        deserialize_with = "deserialize_url",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub hostname: Option<Url>,
}

impl Config {
    pub fn default_path() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("nablaflow").join("config.json"))
    }

    pub async fn load(args: &Args) -> eyre::Result<Self> {
        let mut config = Self::load_from_path(&args.config_path).await?;

        config.aerocloud_token =
            args.aerocloud_token.clone().or(config.aerocloud_token);
        config.hostname = args.hostname.clone().or(config.hostname);

        Ok(config)
    }

    pub async fn write(&self, path: &Path) -> eyre::Result<()> {
        let buf = serde_json::to_vec_pretty(self)
            .wrap_err("serializing config to json")?;

        if let Some(parent) = path.parent() {
            tokio_fs::create_dir_all(parent)
                .await
                .wrap_err("creating parent folders for config")?;
        }

        tokio_fs::write(path, &buf).await.wrap_err("writing config")
    }

    pub fn aerocloud_token_or_fail(&self) -> eyre::Result<&Token> {
        self.aerocloud_token
            .as_ref()
            .ok_or_else(|| eyre::eyre!("no token provided"))
    }

    pub fn hostname(&self) -> Url {
        self.hostname.clone().unwrap_or_else(default_hostname)
    }

    async fn load_from_path(path: &Path) -> eyre::Result<Self> {
        if path.exists() {
            let buf: Vec<u8> = tokio_fs::read(path)
                .await
                .wrap_err_with(|| format!("reading {}", path.display()))?;

            Ok(
                serde_json::from_slice(&buf)
                    .wrap_err("parsing config as json")?,
            )
        } else {
            Ok(Self::default())
        }
    }
}

pub fn default_hostname() -> Url {
    Url::parse(DEFAULT_HOSTNAME).unwrap()
}

#[allow(clippy::ref_option)]
fn serialize_url<S>(v: &Option<Url>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(v) = v {
        ser.serialize_str(v.as_ref())
    } else {
        ser.serialize_none()
    }
}

fn deserialize_url<'de, D>(de: D) -> Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(de)?;
    let url = Url::parse(&s).map_err(serde::de::Error::custom)?;
    Ok(Some(url))
}
