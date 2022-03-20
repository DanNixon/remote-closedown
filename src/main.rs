mod config;
mod event;
mod io;
mod mqtt;
mod output_task;
mod processing;
mod schema;

use crate::{config::Config, event::Event, io::Input};
use anyhow::Result;
use clap::Parser;
use tokio::{signal, sync::broadcast};

#[macro_export]
macro_rules! send_event {
    ($tx:expr, $event:expr) => {
        if let Err(e) = $tx.send($event) {
            log::error!("Failed to send event: {}", e);
        }
    };
}

/// A bridge between Matrix and MQTT
#[derive(Clone, Debug, Parser)]
struct Cli {
    /// MQTT password
    #[clap(long, env = "MQTT_PASSWORD", default_value = "")]
    mqtt_password: String,

    /// Path to configuration file
    #[clap(long, env = "CONFIG_FILE", default_value = "./config.toml")]
    config_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Cli::parse();
    log::debug!("{:?}", args);

    let config = Config::from_file(&args.config_file)?;
    log::debug!("{:?}", config);

    let (tx, _) = broadcast::channel::<Event>(16);

    let mut tasks = vec![
        processing::run(tx.clone(), config.clone())?,
        mqtt::run(tx.clone(), &config.mqtt, &args.mqtt_password).await?,
        output_task::run(tx.clone(), &config)?,
    ];

    if let Some(c) = config.tx_power_status {
        tasks.push(Input::new(&c)?.watch(tx.clone(), |tx, state| {
            crate::send_event!(tx, Event::TxPowerStateChanged(state));
        })?);
    }

    if let Some(c) = config.ptt_status {
        tasks.push(Input::new(&c)?.watch(tx.clone(), |tx, state| {
            crate::send_event!(tx, Event::PttStateChanged(state));
        })?);
    }

    tx.send(Event::SetTxPowerEnable(false))?;
    tx.send(Event::SetPttEnable(false))?;

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            log::error!("Unable to listen for shutdown signal: {}", err);
        }
    }

    log::info! {"Terminating..."};
    tx.send(Event::Exit)?;
    for handle in tasks {
        if let Err(e) = handle.await {
            log::error!("Failed waiting for task to finish: {}", e);
        }
    }

    Ok(())
}
