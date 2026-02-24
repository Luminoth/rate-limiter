use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum RateLimiterError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, RateLimiterError>;
