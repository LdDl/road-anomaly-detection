use redis::{Client};
use std::sync::Arc;

pub struct RedisConnection {
    pub channel_name: String,
    pub client: Arc<Client>,
}

impl RedisConnection {
    pub fn new(host: String, port: i32, db_index: i32, channel_name: String) -> Self {
        let client = Client::open(format!("redis://{}:{}/{}", host, port, db_index)).unwrap();
        return RedisConnection {
            channel_name: channel_name_handler(channel_name),
            client: Arc::new(client),
        };
    }
    pub fn new_with_password(host: String, port: i32, db_index: i32, channel_name: String, password: String) -> Self {
        let client = Client::open(format!(
            "redis://:{}@{}:{}/{}",
            password, host, port, db_index
        )).unwrap();
        return RedisConnection {
            channel_name: channel_name_handler(channel_name), 
            client: Arc::new(client),
        };
    }
    pub fn set_channel(&mut self, _channel_name: String) {
        self.channel_name = channel_name_handler(_channel_name);
    }
}

fn channel_name_handler(s: String) -> String {
    if s.chars().count() != 0 {
        return s;
    }
    "ROAD_ANOMALIES_EVENTS".to_string()
}
