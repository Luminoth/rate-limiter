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

    /// Call when a new request comes in to lazily refill access tokens
    pub async fn on_request(&self, id: impl Into<String>) -> anyhow::Result<()> {
        match self {
            TokenBucketContainer::Memory(c) => c.on_request(id).await,
            TokenBucketContainer::Redis(c) => c.on_request(id).await,
        }
    }

    /// Call before processing a new request to determine if the requester has tokens available
    ///
    /// This will consume one token from the requester's pool
    pub async fn allow_request(&self, id: impl AsRef<str>) -> anyhow::Result<bool> {
        match self {
            TokenBucketContainer::Memory(c) => c.allow_request(id).await,
            TokenBucketContainer::Redis(c) => c.allow_request(id).await,
        }
    }
}
