use std::collections::HashMap;

use chrono::prelude::*;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use super::TokenBucketConfig;

// TODO: implement token bucket expiration

#[derive(Debug)]
pub(crate) struct TokenBucket {
    current_tokens: usize,
    last_refill_time: i64,
}

impl TokenBucket {
    pub(crate) fn new(max_tokens: usize, current_time: i64) -> Self {
        Self {
            current_tokens: max_tokens,
            last_refill_time: current_time,
        }
    }
}

pub struct MemoryTokenBucketContainer<'a> {
    config: &'a TokenBucketConfig,
    buckets: Mutex<HashMap<String, TokenBucket>>,
}

impl<'a> MemoryTokenBucketContainer<'a> {
    pub fn new(config: &'a TokenBucketConfig) -> Self {
        Self {
            config,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub async fn on_request(&self, id: impl Into<String>) -> anyhow::Result<()> {
        let id = id.into();

        // TODO: locking *all* buckets isn't great
        let mut buckets = self.buckets.lock().await;

        let current_time = Utc::now().timestamp();

        let bucket = buckets.entry(id.clone()).or_insert_with(|| {
            debug!("Creating new bucket for {id}");
            TokenBucket::new(self.config.max_tokens, current_time)
        });

        let elapsed = current_time - bucket.last_refill_time;

        // TODO: because this is by seconds, we will miss some partial second refills
        let new_tokens = (elapsed * self.config.refill_rate as i64) as usize;
        if new_tokens > 0 {
            debug!("Adding up to {new_tokens} tokens for {id}");

            bucket.current_tokens = self
                .config
                .max_tokens
                .min(bucket.current_tokens + new_tokens);
            bucket.last_refill_time = current_time;
        }

        Ok(())
    }

    pub async fn allow_request(&self, id: impl AsRef<str>) -> anyhow::Result<bool> {
        let id = id.as_ref();

        // TODO: locking *all* buckets isn't great
        let mut buckets = self.buckets.lock().await;

        let bucket = match buckets.get_mut(id) {
            Some(bucket) => bucket,
            None => {
                warn!("No bucket for {id}!");
                return Ok(false);
            }
        };

        if bucket.current_tokens == 0 {
            debug!("No tokens available for {id}");
            return Ok(false);
        }

        debug!("Consuming one token for {id}");
        bucket.current_tokens -= 1;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use super::*;

    #[tokio::test]
    #[traced_test]
    async fn simple_test() {
        let container = MemoryTokenBucketContainer::new(&TokenBucketConfig {
            max_tokens: 4,
            refill_rate: 1,
        });

        container.on_request("test").await.unwrap();
        assert_eq!(container.allow_request("test").await.unwrap(), true);
    }
}
