mod memory;
mod redis;

use memory::*;
use redis::*;

#[derive(Debug)]
pub struct TokenBucketConfig {
    /// Max tokens the bucket can hold
    max_tokens: usize,

    /// Tokens per second to refill
    refill_rate: usize,
}

pub enum TokenBucketContainer<'a> {
    Memory(MemoryTokenBucketContainer<'a>),
    Redis(RedisTokenBucketContainer<'a>),
}

impl<'a> TokenBucketContainer<'a> {
    pub fn new_memory(config: &'a TokenBucketConfig) -> Self {
        Self::Memory(MemoryTokenBucketContainer::new(config))
    }

    pub fn new_redis(
        config: &'a TokenBucketConfig,
        connection_manager: ::redis::aio::ConnectionManager,
    ) -> Self {
        Self::Redis(RedisTokenBucketContainer::new(config, connection_manager))
    }

    /// Check if the request is allowed.
    ///
    /// This will automatically refill tokens based on elapsed time and consume one token
    /// if available. Returns `true` if the request is allowed, `false` otherwise.
    pub async fn check(&self, id: impl AsRef<str>) -> crate::Result<bool> {
        match self {
            TokenBucketContainer::Memory(c) => c.check(id).await,
            TokenBucketContainer::Redis(c) => c.check(id).await,
        }
    }
}
