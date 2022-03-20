use crate::{config::IoPin, event::Event};
use anyhow::Result;
use sysfs_gpio::{Direction, Pin};
use tokio::{sync::broadcast::Sender, task::JoinHandle};

pub(crate) struct Input {
    pin: Pin,
}

impl Input {
    pub(crate) fn new(config: &IoPin) -> Result<Self> {
        let pin = Pin::new(config.number);
        pin.export()?;
        pin.set_direction(Direction::In)?;
        pin.set_active_low(config.inverted)?;
        Ok(Input { pin })
    }

    pub(crate) fn watch(
        &self,
        tx: Sender<Event>,
        callback: impl Fn(Sender<Event>, bool) + Send + 'static,
    ) -> Result<JoinHandle<()>> {
        let mut rx = tx.subscribe();
        let pin = self.pin;

        Ok(tokio::spawn(async move {
            let mut prev: u8 = 255;

            loop {
                let val = pin.get_value().unwrap();
                if val != prev {
                    callback(tx.clone(), val == 1);
                    prev = val;
                }

                if let Ok(Event::Exit) = rx.try_recv() {
                    log::debug!("Task exit");
                    return;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }))
    }
}
