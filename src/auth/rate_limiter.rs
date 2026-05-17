use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Per-account concurrency limiter.
/// Each account gets a semaphore with `max_concurrent` permits.
/// Permits are automatically released on drop.
#[derive(Clone)]
pub struct RateLimiter {
    /// Inner storage: maps account_id -> (semaphore, current usage count)
    inner: Arc<Mutex<HashMap<Uuid, Arc<Semaphore>>>>,
    max_concurrent: usize,
}

impl RateLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent,
        }
    }

    /// Acquire a concurrency permit for an account.
    /// Blocks until a permit is available.
    /// The returned permit is automatically released on drop.
    pub async fn acquire(&self, account_id: Uuid) -> OwnedSemaphorePermit {
        let sem = {
            let mut map = self.inner.lock().await;
            map.entry(account_id)
                .or_insert_with(|| Arc::new(Semaphore::new(self.max_concurrent)))
                .clone()
        };
        // This await may block; the lock is released before the block.
        let permit = sem.acquire_owned().await;
        // SAFETY: semaphore permits are never 0 (initialized at max_concurrent).
        permit.expect("semaphore closed unexpectedly")
    }

    /// Current usage count for an account (for monitoring).
    pub async fn usage(&self, account_id: Uuid) -> usize {
        let map = self.inner.lock().await;
        map.get(&account_id)
            .map(|s| self.max_concurrent - s.available_permits())
            .unwrap_or(0)
    }
}
