use governor::{
    clock::DefaultClock, state::keyed::HashMapStateStore, Quota, RateLimiter as GovLimiter,
};
use std::num::NonZeroU32;
use uuid::Uuid;

/// Per-account time-window rate limiter using the GCRA algorithm.
///
/// Each account gets an independent rate limit. Requests that exceed
/// the configured rate are queued (not rejected) — the caller blocks
/// until capacity is available.
#[derive(Clone)]
pub struct RateLimiter {
    inner: GovLimiter<Uuid, HashMapStateStore<Uuid>, DefaultClock>,
}

impl RateLimiter {
    /// Creates a rate limiter with the given maximum requests per second
    /// per account.
    pub fn new(max_per_second: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(max_per_second).expect("max_per_second must be non-zero"),
        );
        Self {
            inner: GovLimiter::hashmap(quota),
        }
    }

    /// Blocks until the account has capacity for one request.
    ///
    /// This is a cooperative back-pressure mechanism — callers wait
    /// rather than being rejected. For a server-side limiter, queuing
    /// is preferred over rejection to avoid cascading client retries.
    pub async fn acquire(&self, account_id: Uuid) {
        self.inner.until_key_ready(&account_id).await;
    }
}
