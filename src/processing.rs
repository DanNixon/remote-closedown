use crate::{
    config::Config,
    event::{Event, MqttMessageEvent},
    schema::{Command, Response, Status},
};
use anyhow::Result;
use tokio::{sync::broadcast::Sender, task::JoinHandle};

pub(crate) fn run(tx: Sender<Event>, config: Config) -> Result<JoinHandle<()>> {
    let mut rx = tx.subscribe();

    Ok(tokio::spawn(async move {
        let mut status = Status::default();

        let mut tx_guard_timeout_task: Option<JoinHandle<()>> = None;

        while let Ok(event) = rx.recv().await {
            match event {
                Event::Exit => {
                    log::debug!("Task exit");
                    return;
                }
                Event::MqttMessageReceive(event) => {
                    match serde_json::from_str::<Command>(&event.message) {
                        Ok(cmd) => {
                            log::debug!("Got command message: {:?}", cmd);
                            for cmd_event in cmd.generate_events() {
                                crate::send_event!(tx, cmd_event);
                            }
                        }
                        Err(e) => log::error!("Failed to parse command message: {}", e),
                    }
                }
                Event::TxPowerEnableStateChanged(state) => {
                    status.tx_power_enabled = Some(state);
                    crate::send_event!(tx, Event::SendStatus(None));
                }
                Event::TxPowerStateChanged(state) => {
                    status.tx_power_active = Some(state);
                    crate::send_event!(tx, Event::SendStatus(None));
                }
                Event::PttEnableStateChanged(state) => {
                    status.ptt_enabled = Some(state);
                    crate::send_event!(tx, Event::SendStatus(None));
                }
                Event::PttStateChanged(state) => {
                    status.ptt_active = Some(state);
                    crate::send_event!(tx, Event::SendStatus(None));

                    if let Some(tx_guard_time) = config.tx_guard_time {
                        if let Some(task) = tx_guard_timeout_task {
                            task.abort();
                        }

                        if state {
                            let tx = tx.clone();
                            tx_guard_timeout_task = Some(tokio::spawn(async move {
                                tokio::time::sleep(tx_guard_time).await;
                                crate::send_event!(tx, Event::SetTxPowerEnable(false));
                                crate::send_event!(tx, Event::SetPttEnable(false));
                                crate::send_event!(
                                    tx,
                                    Event::SendStatus(Some(format!(
                                        "TX timed out after {}ms",
                                        tx_guard_time.as_millis()
                                    )))
                                );
                            }));
                        } else {
                            tx_guard_timeout_task = None;
                        }
                    }
                }
                Event::SendStatus(msg) => {
                    if let Err(e) = || -> Result<usize> {
                        Ok(tx.send(Event::MqttMessageSend(MqttMessageEvent::new(
                            &config.mqtt.status_topic,
                            &serde_json::to_string(&Response::new(status.clone(), msg))?,
                        )))?)
                    }() {
                        log::error!("Failed building/sending status message: {}", e);
                    }
                }
                _ => {}
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{sync::broadcast, time::Duration};

    macro_rules! wait_millis {
        ($n: expr) => {
            tokio::time::sleep(Duration::from_millis($n)).await;
        };
    }

    macro_rules! send_event_receive_it_and_yield {
        ($tx: expr, $rx: expr, $event: expr) => {
            $tx.send($event.clone()).unwrap();
            assert_eq!($event, $rx.try_recv().unwrap());
            wait_millis!(10);
        };
    }

    macro_rules! send_tx_on {
        ($tx: expr, $rx: expr) => {
            send_event_receive_it_and_yield!($tx, $rx, Event::PttStateChanged(true));
            assert_eq!(Event::SendStatus(None), $rx.try_recv().unwrap());
            assert!(match $rx.try_recv().unwrap() {
                Event::MqttMessageSend(_) => true,
                _ => false,
            });
        };
    }

    macro_rules! send_tx_off {
        ($tx: expr, $rx: expr) => {
            send_event_receive_it_and_yield!($tx, $rx, Event::PttStateChanged(false));
            assert_eq!(Event::SendStatus(None), $rx.try_recv().unwrap());
            assert!(match $rx.try_recv().unwrap() {
                Event::MqttMessageSend(_) => true,
                _ => false,
            });
        };
    }

    macro_rules! expect_no_event {
        ($rx: expr) => {
            assert!($rx.try_recv().is_err());
        };
    }

    macro_rules! expect_tx_guard_closedown {
        ($rx: expr) => {
            assert_eq!(Event::SetTxPowerEnable(false), $rx.try_recv().unwrap());
            assert_eq!(Event::SetPttEnable(false), $rx.try_recv().unwrap());
            assert_eq!(
                Event::SendStatus(Some("TX timed out after 500ms".to_string())),
                $rx.try_recv().unwrap()
            );
            assert!(match $rx.try_recv().unwrap() {
                Event::MqttMessageSend(_) => true,
                _ => false,
            });
        };
    }

    #[tokio::test]
    async fn set_ptt_mqtt_command() {
        let config = Config::default();
        let (tx, mut rx) = broadcast::channel::<Event>(16);
        let task = run(tx.clone(), config).unwrap();

        send_event_receive_it_and_yield!(
            tx,
            rx,
            Event::MqttMessageReceive(MqttMessageEvent::new("", "{\"enable_ptt\":true}",))
        );

        assert_eq!(Event::SetPttEnable(true), rx.try_recv().unwrap());

        tx.send(Event::Exit).unwrap();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn multiple_mqtt_command() {
        let config = Config::default();
        let (tx, mut rx) = broadcast::channel::<Event>(16);
        let task = run(tx.clone(), config).unwrap();

        send_event_receive_it_and_yield!(
            tx,
            rx,
            Event::MqttMessageReceive(MqttMessageEvent::new(
                "",
                "{\"enable_ptt\":true, \"enable_tx_power\":false}",
            ))
        );

        assert_eq!(Event::SetTxPowerEnable(false), rx.try_recv().unwrap());
        assert_eq!(Event::SetPttEnable(true), rx.try_recv().unwrap());

        tx.send(Event::Exit).unwrap();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn tx_guard_basic() {
        let mut config = Config::default();
        config.tx_guard_time = Some(Duration::from_millis(500));
        let (tx, mut rx) = broadcast::channel::<Event>(16);
        let task = run(tx.clone(), config).unwrap();

        send_tx_on!(tx, rx);

        // Not yet 500ms after PPT was detected as active, this is fine.
        wait_millis!(450);
        expect_no_event!(rx);

        // PTT has been active for more than 500ms, this is not fine.
        wait_millis!(100);
        expect_tx_guard_closedown!(rx);

        tx.send(Event::Exit).unwrap();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn tx_guard_extensive() {
        let mut config = Config::default();
        config.tx_guard_time = Some(Duration::from_millis(500));
        let (tx, mut rx) = broadcast::channel::<Event>(16);
        let task = run(tx.clone(), config).unwrap();

        send_tx_on!(tx, rx);
        wait_millis!(150);
        expect_no_event!(rx);
        wait_millis!(150);
        expect_no_event!(rx);
        send_tx_off!(tx, rx);

        wait_millis!(500);

        send_tx_on!(tx, rx);
        wait_millis!(150);
        expect_no_event!(rx);
        wait_millis!(400);
        expect_tx_guard_closedown!(rx);

        wait_millis!(500);

        send_tx_on!(tx, rx);
        wait_millis!(150);
        expect_no_event!(rx);
        wait_millis!(150);
        expect_no_event!(rx);
        send_tx_off!(tx, rx);

        wait_millis!(500);

        send_tx_on!(tx, rx);
        wait_millis!(150);
        expect_no_event!(rx);
        wait_millis!(400);
        expect_tx_guard_closedown!(rx);

        tx.send(Event::Exit).unwrap();
        task.await.unwrap();
    }
}
