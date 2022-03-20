use crate::event::Event;
use chrono::{offset::Local, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize)]
pub(crate) struct Status {
    pub tx_power_enabled: Option<bool>,
    pub tx_power_active: Option<bool>,
    pub ptt_enabled: Option<bool>,
    pub ptt_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Response {
    pub status: Status,
    pub message: Option<String>,
    pub timestamp: DateTime<Local>,
}

impl Response {
    pub(crate) fn new(status: Status, message: Option<String>) -> Self {
        Self {
            status,
            message,
            timestamp: Local::now(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Command {
    enable_tx_power: Option<bool>,
    enable_ptt: Option<bool>,
}

impl Command {
    pub(crate) fn generate_events(&self) -> Vec<Event> {
        let mut v = Vec::new();

        if let Some(en) = self.enable_tx_power {
            v.push(Event::SetTxPowerEnable(en));
        }

        if let Some(en) = self.enable_ptt {
            v.push(Event::SetPttEnable(en));
        }

        v
    }
}
