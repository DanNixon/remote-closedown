use anyhow::Result;
use serde::Deserialize;
use std::fs;
use tokio::time::Duration;

mod duration_format {
    use serde::{self, Deserialize, Deserializer};
    use tokio::time::Duration;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<u64>::deserialize(deserializer)?.map(Duration::from_millis))
    }
}

#[derive(Clone, Default, Debug, Deserialize)]
pub(crate) struct Mqtt {
    pub broker: String,
    pub client_id: String,

    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,

    pub status_topic: String,
    pub command_topic: String,
}

#[derive(Clone, Default, Debug, Deserialize)]
pub(crate) struct IoPin {
    pub number: u64,

    #[serde(default)]
    pub inverted: bool,
}

#[derive(Clone, Default, Debug, Deserialize)]
pub(crate) struct Config {
    pub mqtt: Mqtt,

    pub tx_power_enable: Option<IoPin>,
    pub tx_power_status: Option<IoPin>,

    pub ptt_enable: Option<IoPin>,
    pub ptt_status: Option<IoPin>,

    #[serde(default, with = "duration_format")]
    pub tx_guard_time: Option<Duration>,
}

impl Config {
    pub fn from_file(filename: &str) -> Result<Self> {
        Ok(toml::from_str(&fs::read_to_string(filename)?)?)
    }
}
