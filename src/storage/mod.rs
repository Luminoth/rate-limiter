mod memory;
mod redis;

use memory::*;
use redis::*;

use crate::TokenBucketConfig;

#[derive(Debug, Clone)]
pub enum Storage<'a> {
    Memory(MemoryStorage<'a>),
    Redis(RedisStorage<'a>),
}

impl<'a> Storage<'a> {
    pub fn new_memory(config: &'a TokenBucketConfig) -> Self {
        Self::Memory(MemoryStorage::new(config))
    }

    pub fn new_redis(
        config: &'a TokenBucketConfig,
        connection_manager: ::redis::aio::ConnectionManager,
    ) -> Self {
        Self::Redis(RedisStorage::new(config, connection_manager))
    }

    pub async fn check(&self, id: impl AsRef<str>) -> crate::Result<bool> {
        match self {
            Self::Memory(c) => Ok(c.check(id).await),
            Self::Redis(c) => c.check(id).await,
        }
    }
}
