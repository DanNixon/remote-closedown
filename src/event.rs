use paho_mqtt::Message;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MqttMessageEvent {
    topic: String,
    pub message: String,
}

impl MqttMessageEvent {
    pub(crate) fn new(topic: &str, message: &str) -> Self {
        Self {
            topic: topic.to_string(),
            message: message.to_string(),
        }
    }
}

impl From<Message> for MqttMessageEvent {
    fn from(msg: Message) -> Self {
        Self {
            topic: msg.topic().to_string(),
            message: msg.payload_str().to_string(),
        }
    }
}

impl From<MqttMessageEvent> for Message {
    fn from(msg: MqttMessageEvent) -> Self {
        Self::new(msg.topic, msg.message, 2)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Event {
    MqttMessageReceive(MqttMessageEvent),
    MqttMessageSend(MqttMessageEvent),
    SetTxPowerEnable(bool),
    TxPowerEnableStateChanged(bool),
    TxPowerStateChanged(bool),
    SetPttEnable(bool),
    PttEnableStateChanged(bool),
    PttStateChanged(bool),
    SendStatus(Option<String>),
    Exit,
}
