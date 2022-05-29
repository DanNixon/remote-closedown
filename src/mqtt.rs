use crate::{
    config::Mqtt,
    event::{Event, MqttMessageEvent},
    schema::{Response, Status},
};
use anyhow::{anyhow, Result};
use paho_mqtt::{
    AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, Message, PersistenceType,
};
use tokio::{
    sync::broadcast::Sender,
    task::JoinHandle,
    time::{self, Duration},
};

pub(crate) async fn run(tx: Sender<Event>, config: &Mqtt) -> Result<JoinHandle<()>> {
    let mut client = AsyncClient::new(
        CreateOptionsBuilder::new()
            .server_uri(&config.broker)
            .client_id(&config.client_id)
            .persistence(PersistenceType::None)
            .finalize(),
    )?;

    let stream = client.get_stream(25);

    let command_topic = config.command_topic.clone();
    client.set_connected_callback(move |c| {
        c.subscribe(&command_topic.clone(), 2);
    });

    client
        .connect(
            ConnectOptionsBuilder::new()
                .clean_session(true)
                .user_name(&config.username)
                .password(&config.password)
                .will_message(Message::new(
                    config.status_topic.as_str(),
                    serde_json::to_string(&Response::new(
                        Status::default(),
                        Some("Station controller has gone offline".to_string()),
                    ))?,
                    0,
                ))
                .finalize(),
        )
        .wait()?;

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

            match stream.try_recv() {
                Ok(Some(msg)) => {
                    log::info! {"Received message on topic \"{}\"", msg.topic()};
                    crate::send_event!(tx, Event::MqttMessageReceive(MqttMessageEvent::from(msg)));
                }
                Ok(None) => {
                    if let Err(e) = try_reconnect(&client).await {
                        log::error!("Failed to reconnect: {}", e);
                        tx.send(Event::Exit).unwrap();
                    }
                }
                Err(_) => {}
            }

            beat.tick().await;
        }
    }))
}

async fn try_reconnect(c: &AsyncClient) -> Result<()> {
    for i in 0..300 {
        log::info!("Attempting reconnection {}...", i);
        match c.reconnect().await {
            Ok(_) => {
                log::info!("Reconnection successful");
                return Ok(());
            }
            Err(e) => {
                log::error!("Reconnection failed: {}", e);
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow!("Failed to reconnect to broker"))
}
