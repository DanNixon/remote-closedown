use crate::{
    config::Mqtt,
    event::{Event, MqttMessageEvent},
    schema::{Response, Status},
};
use anyhow::Result;
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

    {
        let command_topic = config.command_topic.clone();
        let tx = tx.clone();

        client.set_connected_callback(move |c| {
            log::info!("Connected to broker");

            c.subscribe(command_topic.clone(), 2);

            crate::send_event!(
                tx,
                Event::SendStatus(Some("Station controller is now online".to_string()))
            );
        });
    }

    let response = client
        .connect(
            ConnectOptionsBuilder::new()
                .clean_session(true)
                .automatic_reconnect(Duration::from_secs(1), Duration::from_secs(5))
                .keep_alive_interval(Duration::from_secs(5))
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

    log::info!(
        "Using MQTT version {}",
        response.connect_response().unwrap().mqtt_version
    );

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
