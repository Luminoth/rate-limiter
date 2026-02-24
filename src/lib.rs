#![deny(warnings)]

mod error;
mod storage;

pub use error::*;
use storage::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TokenBucketConfig {
    /// Max tokens the bucket can hold
    max_tokens: usize,

    /// Tokens per second to refill
    refill_rate: usize,
}

#[derive(Debug, Clone)]
pub struct RateLimiter<'a> {
    storage: Storage<'a>,
}

impl<'a> RateLimiter<'a> {
    pub fn new_memory(config: &'a TokenBucketConfig) -> Self {
        Self {
            storage: Storage::new_memory(config),
        }
    }

    pub fn new_redis(
        config: &'a TokenBucketConfig,
        conection_manager: ::redis::aio::ConnectionManager,
    ) -> Self {
        Self {
            storage: Storage::new_redis(config, conection_manager),
        }
    }

    /// Check if the request is allowed.
    ///
    /// This will automatically refill tokens based on elapsed time and consume one token
    /// if available. Returns `true` if the request is allowed, `false` otherwise.
    pub async fn check(&self, id: impl AsRef<str>) -> crate::Result<bool> {
        self.storage.check(id).await
    }
}
