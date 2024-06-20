use crate::events::EventInfo;
use redis::RedisError;

#[derive(Debug)]
pub struct PublisherInternalError(String);

#[derive(Debug)]
pub enum PublisherError{
    Er(PublisherInternalError),
    RedisErr(RedisError),
}

impl From<PublisherInternalError> for PublisherError {
    fn from(e: PublisherInternalError) -> Self {
        PublisherError::Er(e)
    }
}

impl From<RedisError> for PublisherError {
    fn from(e: RedisError) -> Self {
        PublisherError::RedisErr(e)
    }
}

impl std::fmt::Display for PublisherInternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublisherInternalError: {}", self.0)
    }
}

pub trait PublisherTrait {
    fn publish(&self, event: &EventInfo) -> Result<(), PublisherError>;
}
