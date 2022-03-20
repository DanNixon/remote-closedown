use crate::{
    config::Mqtt,
    event::{Event, MqttMessageEvent},
};
use anyhow::Result;
use paho_mqtt::{AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder};
use std::env;
use tokio::{
    sync::broadcast::Sender,
    task::JoinHandle,
    time::{self, Duration},
};

pub(crate) async fn run(
    tx: Sender<Event>,
    config: &Mqtt,
    password: &str,
) -> Result<JoinHandle<()>> {
    let mut client = AsyncClient::new(
        CreateOptionsBuilder::new()
            .server_uri(&config.broker)
            .client_id(&config.client_id)
            .persistence(env::temp_dir())
            .finalize(),
    )?;

    let stream = client.get_stream(25);

    client
        .connect(
            ConnectOptionsBuilder::new()
                .user_name(&config.username)
                .password(password)
                .finalize(),
        )
        .wait()?;

    client.subscribe(&config.command_topic, 2).await?;

    let mut rx = tx.subscribe();

    Ok(tokio::spawn(async move {
        let mut beat = time::interval(Duration::from_millis(100));

        loop {
            if let Ok(event) = rx.try_recv() {
                match event {
                    Event::Exit => {
                        log::debug!("Task exit");
                        return;
                    }
                    Event::MqttMessageSend(msg) => match client.try_publish(msg.into()) {
                        Ok(delivery_token) => {
                            if let Err(e) = delivery_token.wait() {
                                log::error!("Error sending message: {}", e);
                            }
                        }
                        Err(e) => log::error!("Error creating/queuing the message: {}", e),
                    },
                    _ => {}
                }
            }

            if let Ok(Some(msg)) = stream.try_recv() {
                log::info! {"Received message on topic \"{}\"", msg.topic()};
                crate::send_event!(tx, Event::MqttMessageReceive(MqttMessageEvent::from(msg)));
            }

            beat.tick().await;
        }
    }))
}
