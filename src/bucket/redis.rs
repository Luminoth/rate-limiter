use chrono::prelude::*;

use super::TokenBucketConfig;

pub struct RedisTokenBucketContainer<'a> {
    config: &'a TokenBucketConfig,
    connection_manager: redis::aio::ConnectionManager,
}

impl<'a> RedisTokenBucketContainer<'a> {
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

        let script = r"
local key = KEYS[1]
local max_tokens = tonumber(ARGV[1])
local refill_rate = tonumber(ARGV[2])
local current_time = tonumber(ARGV[3])

-- Get current bucket state
local bucket = redis.call('HMGET', key, 'current_tokens', 'last_refill_time')
local current_tokens = tonumber(bucket[1])
local last_refill_time = tonumber(bucket[2])

-- Initialize bucket if it does not exist
if current_tokens == nil then
    current_tokens = max_tokens
    last_refill_time = current_time
end

-- Refill logic
local elapsed = math.max(0, current_time - last_refill_time)
if elapsed > 0 then
    local new_tokens = (elapsed / 1000) * refill_rate
    current_tokens = math.min(max_tokens, current_tokens + new_tokens)
    last_refill_time = current_time
end

local allowed = 0
if current_tokens >= 1 then
    current_tokens = current_tokens - 1
    allowed = 1
end

-- Save state
redis.call('HMSET', key, 'current_tokens', current_tokens, 'last_refill_time', last_refill_time)
redis.call('EXPIRE', key, math.ceil(max_tokens / refill_rate) + 1)

return allowed
        ";

        // TODO: should only do this once, not on each call
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
