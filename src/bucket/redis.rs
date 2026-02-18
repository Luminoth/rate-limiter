use chrono::prelude::*;

use super::TokenBucketConfig;

// TODO: this is totally untested

pub struct RedisTokenBucketContainer<'a> {
    #[allow(dead_code)]
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

    pub async fn on_request(&self, id: impl Into<String>) -> anyhow::Result<()> {
        let id = id.into();

        let script_code = r"
local key = KEYS[1]
local max_tokens = tonumber(ARGV[1])   -- Max tokens in the bucket
local refill_rate = tonumber(ARGV[2])  -- Tokens added per second
local current_time = tonumber(ARGV[3])

-- Get current bucket state
local bucket = redis.call('HMGET', key, 'current_tokens', 'last_refill_time')
local current_tokens = tonumber(bucket[1])
local last_refill_time = tonumber(bucket[2])

-- Initialize bucket if it does not exist
if tokens == nil then
    current_tokens = max_tokens
    last_refill_time = current_time
end

-- Calculate tokens to add based on elapsed time
local elapsed = current_time - last_refill_time
local new_tokens = elapsed * refill_rate

if new_tokens > 0 then
    current_tokens = math.min(max_tokens, current_tokens + new_tokens)

    -- Update the bucket state
    redis.call('HMSET', key, 'current_tokens', current_tokens, 'last_refill_time', current_time)
    redis.call('EXPIRE', key, math.ceil(max_tokens / refill_rate) + 1)
end
    ";
        let script = redis::Script::new(script_code);

        let mut conn = self.connection_manager.clone();
        let _: () = script
            .key(id) // TODO: probably want to use a better "key" than *just* the id
            .arg(self.config.max_tokens)
            .arg(self.config.refill_rate)
            .arg(Utc::now().timestamp())
            .invoke_async(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn allow_request(&self, id: impl AsRef<str>) -> anyhow::Result<bool> {
        let id = id.as_ref();

        let script_code = r"
local key = KEYS[1]

-- Get current bucket state
local bucket = redis.call('HMGET', key, 'current_tokens')
local current_tokens = tonumber(bucket[1])

if current_tokens == nil then
    return false
end

if current_tokens == 0 then
    return false
end

-- Update the bucket state
redis.call('HMSET', key, 'current_tokens', current_tokens - 1)

return true
    ";
        let script = redis::Script::new(script_code);

        let mut conn = self.connection_manager.clone();
        let allow: bool = script
            .key(id) // TODO: probably want to use a better "key" than *just* the id
            .invoke_async(&mut conn)
            .await?;

        Ok(allow)
    }
}
