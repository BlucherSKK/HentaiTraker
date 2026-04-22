use deadpool_redis::{Config, Pool, Runtime, Connection};
use deadpool_redis::redis::AsyncCommands;

/// Async Redis client backed by a deadpool connection pool.
/// All operations are fire-and-forget on failure — a broken Redis
/// never propagates errors to the caller, the app just skips the cache.
#[derive(Clone)]
pub struct RedisClient {
    pool: Pool,
}

#[derive(Debug)]
pub enum RedisInitError {
    CreatePool(deadpool_redis::CreatePoolError),
}

impl std::fmt::Display for RedisInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisInitError::CreatePool(e) => write!(f, "redis pool init: {e}"),
        }
    }
}

impl std::error::Error for RedisInitError {}

impl RedisClient {
    pub fn init(url: &str) -> Result<Self, RedisInitError> {
        let cfg = Config::from_url(url);
        let pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .map_err(RedisInitError::CreatePool)?;
        Ok(Self { pool })
    }

    async fn acquire(&self) -> Option<Connection> {
        match self.pool.get().await {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("redis: pool acquire failed: {e}");
                None
            }
        }
    }

    /// GET — returns `None` on miss or any error.
    pub async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.acquire().await?;
        match conn.get::<_, Option<String>>(key).await {
            Ok(v) => v,
            Err(e) => {
                warn!("redis GET {key} failed: {e}");
                None
            }
        }
    }

    /// SET with TTL in seconds. Silent on failure.
    pub async fn set_ex(&self, key: &str, value: &str, ttl_secs: u64) {
        let Some(mut conn) = self.acquire().await else { return };
        if let Err(e) = conn.set_ex::<_, _, ()>(key, value, ttl_secs).await {
            warn!("redis SET {key} failed: {e}");
        }
    }

    /// DEL. Silent on failure.
    pub async fn del(&self, key: &str) {
        let Some(mut conn) = self.acquire().await else { return };
        if let Err(e) = conn.del::<_, ()>(key).await {
            warn!("redis DEL {key} failed: {e}");
        }
    }

    /// Atomic counter increment.
    /// Sets TTL only on first increment (count == 1) so the counter
    /// automatically expires after `ttl_secs` of inactivity.
    /// Returns 0 on any error so callers never block.
    pub async fn incr_counter(&self, key: &str, ttl_secs: u64) -> u64 {
        let Some(mut conn) = self.acquire().await else { return 0 };

        let count: u64 = match conn.incr(key, 1u64).await {
            Ok(v) => v,
            Err(e) => {
                warn!("redis INCR {key} failed: {e}");
                return 0;
            }
        };

        if count == 1 {
            let _: Result<(), _> = deadpool_redis::redis::cmd("EXPIRE")
            .arg(key)
            .arg(ttl_secs)
            .query_async::<()>(&mut *conn)
            .await;
        }

        count
    }
}
