use crate::publisher::publisher::{PublisherTrait, PublisherError};
use crate::events::events::EventInfo;

use redis::{Client, Commands};
use std::sync::Arc;

pub struct RedisConnection {
    pub channel_name: String,
    pub client: Arc<Client>,
}

impl RedisConnection {
    pub fn new(host: String, port: i32, db_index: i32, channel_name: String) -> Result<Box<dyn PublisherTrait>, redis::RedisError> {
        let client = Client::open(format!("redis://{}:{}/{}", host, port, db_index))?;
        Ok(Box::new(RedisConnection {
            channel_name: channel_name_handler(channel_name),
            client: Arc::new(client),
        }))
    }
    pub fn new_with_password(host: String, port: i32, db_index: i32, channel_name: String, password: String) -> Result<Box<dyn PublisherTrait>, redis::RedisError> {
        let client = Client::open(format!(
            "redis://:{}@{}:{}/{}",
            password, host, port, db_index
        ))?;
        Ok(Box::new(RedisConnection {
            channel_name: channel_name_handler(channel_name), 
            client: Arc::new(client),
        }))
    }
    pub fn new_with_username_password(host: String, port: i32, db_index: i32, channel_name: String, username: String, password: String) -> Result<Box<dyn PublisherTrait>, redis::RedisError> {
        let client = Client::open(format!(
            "redis://{}:{}@{}:{}/{}",
            username, password, host, port, db_index
        ))?;
        Ok(Box::new(RedisConnection {
            channel_name: channel_name_handler(channel_name), 
            client: Arc::new(client),
        }))
    }
    pub fn set_channel(&mut self, _channel_name: String) {
        self.channel_name = channel_name_handler(_channel_name);
    }
}

impl PublisherTrait for RedisConnection {
    fn publish(&self, event_info: &EventInfo) -> Result<(), PublisherError> {
        let event_id = event_info.get_id();
        let object_id = event_info.get_object_id();
        println!("Trying to send event: {}. Object ID: '{}'. Time: {}", event_id, object_id, chrono::Utc::now());
        let mut redis_conn = self.client.get_connection()?;
        let event_json_str = serde_json::to_string(event_info)?;
        redis_conn.publish(self.channel_name.to_owned(), event_json_str)?;
        println!("Success for sending event: '{}'. Object ID: '{}'. Time: {}", event_id, object_id, chrono::Utc::now());
        Ok(())
    }
}

fn channel_name_handler(s: String) -> String {
    if s.chars().count() != 0 {
        return s;
    }
    "ROAD_ANOMALIES_EVENTS".to_string()
}
