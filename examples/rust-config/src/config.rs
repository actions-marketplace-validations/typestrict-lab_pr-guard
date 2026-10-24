use std::time::Duration;

use serde::Deserialize;

use crate::duration::parse_duration;

/// Service settings read from `<config dir>/<profile>.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_timeout", deserialize_with = "de_duration")]
    pub request_timeout: Duration,
    pub database_url: String,
}

fn default_listen_addr() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_workers() -> usize {
    4
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn de_duration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    let raw = String::deserialize(d)?;
    parse_duration(&raw).map_err(serde::de::Error::custom)
}
