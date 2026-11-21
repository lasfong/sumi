use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::error::{DataError, Result};

/// Configuration for [`TokenBucketRateLimiter`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateLimitConfig {
    /// Maximum burst tokens available immediately.
    pub burst_capacity: f64,
    /// Sustained token refill rate (requests per second).
    pub refill_per_sec: f64,
    /// Maximum number of concurrent in-flight network requests guarded by the semaphore.
    pub max_concurrent: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::tradingview_default()
    }
}

impl RateLimitConfig {
    /// Conservative pacing profile tuned for TradingView WebSocket/REST endpoints
    /// so batch `Scanner` scans across hundreds of symbols avoid IP throttling.
    #[must_use]
    pub const fn tradingview_default() -> Self {
        Self {
            burst_capacity: 6.0,
            refill_per_sec: 3.0,
            max_concurrent: 4,
        }
    }

    /// High-throughput profile for unit tests or local mock endpoints.
    #[must_use]
    pub const fn fast_test() -> Self {
        Self {
            burst_capacity: 100.0,
            refill_per_sec: 1_000.0,
            max_concurrent: 16,
        }
    }
}

/// Snapshot of runtime counters exposed by [`TokenBucketRateLimiter`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateLimitStats {
    pub available_tokens: f64,
    pub available_permits: usize,
    pub max_concurrent: usize,
    pub total_acquired: u64,
    pub total_waits: u64,
}

#[derive(Debug)]
struct BucketState {
    tokens: f64,
    capacity: f64,
    refill_per_sec: f64,
    last_refill: Instant,
    total_acquired: u64,
    total_waits: u64,
}

impl BucketState {
    fn refill(&mut self, now: Instant) {
        let elapsed = now
            .saturating_duration_since(self.last_refill)
            .as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
            self.last_refill = now;
        }
    }
}

/// Guard holding a concurrency slot in [`TokenBucketRateLimiter`].
///
/// The concurrency permit is automatically released on drop.
#[derive(Debug)]
pub struct RateLimitPermit {
    _permit: OwnedSemaphorePermit,
}

/// Async token-bucket rate limiter paired with a bounded concurrency [`Semaphore`].
///
/// Protects upstream market data servers against request bursts and unbounded parallelism
/// during multi-symbol historical downloads and scanner sweeps.
#[derive(Debug, Clone)]
pub struct TokenBucketRateLimiter {
    bucket: Arc<Mutex<BucketState>>,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl TokenBucketRateLimiter {
    /// Constructs a new rate limiter from [`RateLimitConfig`].
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        let capacity = if config.burst_capacity.is_finite() && config.burst_capacity >= 1.0 {
            config.burst_capacity
        } else {
            1.0
        };
        let refill_per_sec = if config.refill_per_sec.is_finite() && config.refill_per_sec > 0.0 {
            config.refill_per_sec
        } else {
            1.0
        };
        let max_concurrent = config.max_concurrent.max(1);

        Self {
            bucket: Arc::new(Mutex::new(BucketState {
                tokens: capacity,
                capacity,
                refill_per_sec,
                last_refill: Instant::now(),
                total_acquired: 0,
                total_waits: 0,
            })),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// Acquires both a concurrency semaphore permit and one rate-limit token.
    pub async fn acquire(&self) -> Result<RateLimitPermit> {
        let permit = Arc::clone(&self.semaphore)
            .acquire_owned()
            .await
            .map_err(|_| DataError::RateLimited("semaphore closed".to_owned()))?;
        self.wait_for_token(None).await?;
        Ok(RateLimitPermit { _permit: permit })
    }

    /// Like [`Self::acquire`], but aborts immediately with [`DataError::Cancelled`] if `cancel` triggers.
    pub async fn acquire_cancellable(&self, cancel: &CancellationToken) -> Result<RateLimitPermit> {
        let permit = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(DataError::Cancelled),
            res = Arc::clone(&self.semaphore).acquire_owned() => {
                res.map_err(|_| DataError::RateLimited("semaphore closed".to_owned()))?
            }
        };
        self.wait_for_token(Some(cancel)).await?;
        Ok(RateLimitPermit { _permit: permit })
    }

    async fn wait_for_token(&self, cancel: Option<&CancellationToken>) -> Result<()> {
        loop {
            let sleep_dur = {
                let mut state = self
                    .bucket
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                state.refill(Instant::now());
                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    state.total_acquired = state.total_acquired.saturating_add(1);
                    return Ok(());
                }
                let deficit = (1.0 - state.tokens).max(0.0);
                state.total_waits = state.total_waits.saturating_add(1);
                Duration::from_secs_f64((deficit / state.refill_per_sec).max(0.001))
            };

            if let Some(token) = cancel {
                tokio::select! {
                    biased;
                    () = token.cancelled() => return Err(DataError::Cancelled),
                    () = tokio::time::sleep(sleep_dur) => {}
                }
            } else {
                tokio::time::sleep(sleep_dur).await;
            }
        }
    }

    /// Attempts to consume a single token without waiting (useful for unit tests and non-blocking probes).
    #[must_use]
    pub fn try_acquire_token(&self) -> bool {
        let mut state = self
            .bucket
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.refill(Instant::now());
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            state.total_acquired = state.total_acquired.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Returns a point-in-time snapshot of limiter counters.
    #[must_use]
    pub fn stats(&self) -> RateLimitStats {
        let mut state = self
            .bucket
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.refill(Instant::now());
        RateLimitStats {
            available_tokens: state.tokens,
            available_permits: self.semaphore.available_permits(),
            max_concurrent: self.max_concurrent,
            total_acquired: state.total_acquired,
            total_waits: state.total_waits,
        }
    }

    /// Configured maximum concurrent permits.
    #[must_use]
    pub const fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[tokio::test]
    async fn token_bucket_paces_after_burst_exhaustion() {
        let limiter = TokenBucketRateLimiter::new(RateLimitConfig {
            burst_capacity: 2.0,
            refill_per_sec: 40.0, // 25ms per token
            max_concurrent: 4,
        });

        let start = Instant::now();
        // First 2 tokens should succeed immediately from burst capacity.
        let p1 = limiter.acquire().await.unwrap();
        let p2 = limiter.acquire().await.unwrap();
        drop((p1, p2));

        // 3rd and 4th tokens must wait for refill (~50ms total).
        let p3 = limiter.acquire().await.unwrap();
        let p4 = limiter.acquire().await.unwrap();
        drop((p3, p4));

        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(35),
            "expected pacing wait >= 35ms after burst, got {elapsed:?}"
        );
        let stats = limiter.stats();
        assert_eq!(stats.total_acquired, 4);
        assert!(stats.total_waits >= 2);
    }

    #[tokio::test]
    async fn semaphore_enforces_max_concurrency() {
        let limiter = Arc::new(TokenBucketRateLimiter::new(RateLimitConfig {
            burst_capacity: 20.0,
            refill_per_sec: 500.0,
            max_concurrent: 2,
        }));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..8 {
            let lim = Arc::clone(&limiter);
            let act = Arc::clone(&active);
            let pk = Arc::clone(&peak);
            handles.push(tokio::spawn(async move {
                let _permit = lim.acquire().await.unwrap();
                let cur = act.fetch_add(1, Ordering::SeqCst) + 1;
                pk.fetch_max(cur, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(15)).await;
                act.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }
        assert!(
            peak.load(Ordering::SeqCst) <= 2,
            "peak concurrency exceeded semaphore bound"
        );
        assert_eq!(limiter.stats().available_permits, 2);
    }

    #[tokio::test]
    async fn acquire_cancellable_aborts_cleanly_while_waiting() {
        let limiter = TokenBucketRateLimiter::new(RateLimitConfig {
            burst_capacity: 1.0,
            refill_per_sec: 0.2, // 5s per token
            max_concurrent: 1,
        });
        let _first = limiter.acquire().await.unwrap();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            cancel_clone.cancel();
        });
        let err = limiter.acquire_cancellable(&cancel).await.unwrap_err();
        assert!(matches!(err, DataError::Cancelled));
    }
}
