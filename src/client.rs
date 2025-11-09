use std::time::UNIX_EPOCH;

use rumqttc::{Client, ClientError, QoS};
use serde_json::json;

use crate::LogItem;

pub struct LogqttClient {
    client: Client,
    base_topic: String,
}

impl LogqttClient {
    pub fn new(client: Client, base_topic: String) -> Self {
        Self { client, base_topic }
    }

    pub fn push(&mut self, log_item: LogItem) -> Result<(), ClientError> {
        let topic = format!(
            "{}/{}/{}/{}",
            self.base_topic,
            log_item.hostname,
            log_item.unit,
            log_item.level.as_ref()
        );

        let timestamp = if let Ok(timestamp) = log_item.timestamp.duration_since(UNIX_EPOCH) {
            timestamp.as_micros()
        } else {
            log::warn!(
                "Time overflow when processing log item timestamp: {:?}",
                log_item.timestamp
            );
            0
        };

        let payload = serde_json::to_string(&json!({
            "message" : log_item.message,
            "timestamp" : timestamp
        }))
        // serialization should never fail; safe to unwrap
        .expect("failed to serialize JSON");

        self.client.publish(topic, QoS::AtLeastOnce, false, payload)
    }
}
