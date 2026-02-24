use chrono::prelude::*;

use crate::TokenBucketConfig;

#[derive(Debug, Clone)]
pub struct RedisStorage<'a> {
    config: &'a TokenBucketConfig,
    connection_manager: redis::aio::ConnectionManager,
}

impl<'a> RedisStorage<'a> {
    pub fn new(
        config: &'a TokenBucketConfig,
        connection_manager: redis::aio::ConnectionManager,
    ) -> Self {
        Self {
            config,
            connection_manager,
        }
    }

    pub async fn check(&self, id: impl AsRef<str>) -> crate::Result<bool> {
        let id = id.as_ref();
        let current_time_ms = Utc::now().timestamp_millis();

        // TODO: should only do this once, not on each call
        let script = include_str!("check.lua");
        let script = redis::Script::new(script);

        let mut conn = self.connection_manager.clone();

        let key = format!("rate_limit:{}", id);

        let result: i32 = script
            .key(key)
            .arg(self.config.max_tokens as f64)
            .arg(self.config.refill_rate as f64)
            .arg(current_time_ms)
            .invoke_async(&mut conn)
            .await?;

        Ok(result == 1)
    }
}
