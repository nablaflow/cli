use reqwest::Url;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub token: String,

    #[serde(
        serialize_with = "serialize_url",
        deserialize_with = "deserialize_url"
    )]
    pub hostname: Url,
}

pub fn path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("nablaflow").join("config.toml"))
}

fn serialize_url<S>(v: &Url, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(v.as_ref())
}

fn deserialize_url<'de, D>(de: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(de)?;
    Url::parse(&s).map_err(serde::de::Error::custom)
}
