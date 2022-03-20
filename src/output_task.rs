use crate::{
    config::{Config, IoPin},
    event::Event,
    io::Output,
};
use anyhow::Result;
use tokio::{sync::broadcast::Sender, task::JoinHandle};

fn output_or_none(config: &Option<IoPin>) -> Result<Option<Output>> {
    Ok(match config {
        Some(config) => Some(Output::new(config)?),
        None => None,
    })
}

pub(crate) fn run(tx: Sender<Event>, config: &Config) -> Result<JoinHandle<()>> {
    let mut rx = tx.subscribe();

    let tx_power_enable_output = output_or_none(&config.tx_power_enable)?;
    let ptt_enable_output = output_or_none(&config.ptt_enable)?;

    Ok(tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                Event::Exit => {
                    log::debug!("Task exit");
                    return;
                }
                Event::SetTxPowerEnable(state) => {
                    log::info!("Request setting TX power enable to {}", state);
                    if let Some(ref output) = tx_power_enable_output {
                        match output.set(state) {
                            Ok(_) => {
                                crate::send_event!(tx, Event::TxPowerEnableStateChanged(state));
                            }
                            Err(e) => {
                                log::error!("Failed to set TX power enable: {}", e);
                                crate::send_event!(
                                    tx,
                                    Event::SendStatus(Some(
                                        "Failed to set TX power enable".to_string(),
                                    ))
                                );
                            }
                        }
                    }
                }
                Event::SetPttEnable(state) => {
                    log::info!("Request setting PTT enable to {}", state);
                    if let Some(ref output) = ptt_enable_output {
                        match output.set(state) {
                            Ok(_) => {
                                crate::send_event!(tx, Event::PttEnableStateChanged(state));
                            }
                            Err(e) => {
                                log::error!("Failed to set PTT enable: {}", e);
                                crate::send_event!(
                                    tx,
                                    Event::SendStatus(
                                        Some("Failed to set PTT enable".to_string(),)
                                    )
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }))
}
