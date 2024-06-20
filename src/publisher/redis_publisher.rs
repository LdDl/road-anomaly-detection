use crate::publisher::publisher::{PublisherTrait, PublisherError};
use crate::events::events::EventInfo;

use redis::{Client, Commands};
use std::sync::Arc;

pub struct RedisConnection {
    pub channel_name: String,
    pub client: Arc<Client>,
}

impl RedisConnection {
    pub fn new(host: String, port: i32, db_index: i32, channel_name: String) -> Box<dyn PublisherTrait> {
        let client = Client::open(format!("redis://{}:{}/{}", host, port, db_index)).unwrap();
        return Box::new(RedisConnection {
            channel_name: channel_name_handler(channel_name),
            client: Arc::new(client),
        });
    }
    pub fn new_with_password(host: String, port: i32, db_index: i32, channel_name: String, password: String) -> Box<dyn PublisherTrait> {
        let client = Client::open(format!(
            "redis://:{}@{}:{}/{}",
            password, host, port, db_index
        )).unwrap();
        return Box::new(RedisConnection {
            channel_name: channel_name_handler(channel_name), 
            client: Arc::new(client),
        });
    }
    pub fn set_channel(&mut self, _channel_name: String) {
        self.channel_name = channel_name_handler(_channel_name);
    }
}

impl PublisherTrait for RedisConnection {
    fn publish(&self, event_info: &EventInfo) -> Result<(), PublisherError> {
        println!("Trying to send data...");
        let mut redis_conn = match self.client.get_connection() {
            Ok(_conn) => _conn,
            Err(_err) => {
                return Err(_err.into());
            }
        };
        todo!("Need to implement publisher for Redis");
        println!("...Success");
        Ok(())
    }
}

fn channel_name_handler(s: String) -> String {
    if s.chars().count() != 0 {
        return s;
    }
    "ROAD_ANOMALIES_EVENTS".to_string()
}
