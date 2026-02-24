use std::sync::Arc;
use std::time::Duration;

use chrono::prelude::*;
use moka::future::Cache;
use parking_lot::Mutex;
use tracing::debug;

use crate::TokenBucketConfig;

#[derive(Debug, Copy, Clone)]
struct TokenBucket {
    current_tokens: f64,   // float for precision refill
    last_refill_time: i64, // milliseconds
}

impl TokenBucket {
    fn new(max_tokens: usize, current_time: i64) -> Self {
        Self {
            current_tokens: max_tokens as f64,
            last_refill_time: current_time,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStorage<'a> {
    config: &'a TokenBucketConfig,
    buckets: Cache<String, Arc<Mutex<TokenBucket>>>,
}

impl<'a> MemoryStorage<'a> {
    pub fn new(config: &'a TokenBucketConfig) -> Self {
        let time_to_fill_seconds =
            (config.max_tokens as f64 / config.refill_rate as f64).ceil() as u64;
        let tti = Duration::from_secs(time_to_fill_seconds + 1);

        let buckets = Cache::builder().time_to_idle(tti).build();

        Self { config, buckets }
    }

    pub async fn check(&self, id: impl AsRef<str>) -> crate::Result<bool> {
        let id = id.as_ref().to_string();
        let current_time = Utc::now().timestamp_millis();

        let bucket_mutex = self
            .buckets
            .get_with(id.clone(), async {
                debug!("Creating new bucket for {id}");
                Arc::new(Mutex::new(TokenBucket::new(
                    self.config.max_tokens,
                    current_time,
                )))
            })
            .await;

        let mut bucket = bucket_mutex.lock();

        let elapsed = current_time - bucket.last_refill_time;
        if elapsed > 0 {
            let new_tokens = (elapsed as f64 / 1000.0) * self.config.refill_rate as f64;
            debug!("Adding up to {new_tokens} tokens for {id}");

            bucket.current_tokens =
                (bucket.current_tokens + new_tokens).min(self.config.max_tokens as f64);
            bucket.last_refill_time = current_time;
        }

        if bucket.current_tokens >= 1.0 {
            debug!("Consuming one token for {id}");
            bucket.current_tokens -= 1.0;
            Ok(true)
        } else {
            debug!("No tokens available for {id}");
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use super::*;

    #[tokio::test]
    #[traced_test]
    async fn simple_test() {
        let storage = MemoryStorage::new(&TokenBucketConfig {
            max_tokens: 4,
            refill_rate: 1,
        });

        assert_eq!(storage.check("test").await.unwrap(), true);
        assert_eq!(storage.check("test").await.unwrap(), true);
        assert_eq!(storage.check("test").await.unwrap(), true);
        assert_eq!(storage.check("test").await.unwrap(), true);
        assert_eq!(storage.check("test").await.unwrap(), false);
    }

    #[tokio::test]
    #[traced_test]
    async fn refill_test() {
        let storage = MemoryStorage::new(&TokenBucketConfig {
            max_tokens: 1,
            refill_rate: 1,
        });

        assert_eq!(storage.check("refill").await.unwrap(), true);
        assert_eq!(storage.check("refill").await.unwrap(), false);

        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        assert_eq!(storage.check("refill").await.unwrap(), true);
    }

    #[tokio::test]
    #[traced_test]
    async fn ttl_test() {
        let storage = MemoryStorage::new(&TokenBucketConfig {
            max_tokens: 1,
            refill_rate: 1,
        });

        assert_eq!(storage.check("ttl").await.unwrap(), true);
        assert_eq!(storage.check("ttl").await.unwrap(), false);

        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

        assert_eq!(storage.check("ttl").await.unwrap(), true);
    }
}
