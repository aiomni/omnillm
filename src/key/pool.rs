//! Key pool with randomised selection, CAS reservation, and circuit breaker.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use rand::Rng;

use super::inner::{now_millis, KeyInner, STATE_DEAD};
use super::lease::KeyLease;
use crate::config::PoolConfig;
use crate::error::ApiError;

/// A pool of API keys for a single provider/model combination.
///
/// Handles key selection (randomised first-fit), atomic TPM reservation,
/// error-driven state transitions, and circuit breaking.
pub struct KeyPool {
    keys: Vec<Arc<KeyInner>>,
    config: PoolConfig,
}

impl KeyPool {
    /// Create a new pool from the given keys and configuration.
    pub(crate) fn new(keys: Vec<Arc<KeyInner>>, config: PoolConfig) -> Self {
        Self { keys, config }
    }

    /// Select a key and atomically reserve `estimated_tokens` of TPM quota.
    ///
    /// Strategy: randomized start index + first-fit, capped at
    /// [`PoolConfig::max_cas_attempts`].
    ///
    /// - Avoids thundering herd: concurrent requests start from different indices.
    /// - On CAS failure, skips to the next key instead of spinning.
    /// - Hard cap prevents long-tail CPU spin under extreme contention.
    /// - O(1) amortized when keys have sufficient capacity.
    pub(crate) fn acquire(&self, estimated_tokens: u32) -> Option<KeyLease> {
        if self.keys.is_empty() {
            return None;
        }

        let n = self.keys.len();
        let start = rand::thread_rng().gen_range(0..n);
        let mut attempts = 0;

        for i in 0..n {
            if attempts >= self.config.max_cas_attempts {
                break; // hard cap — avoid long-tail spin
            }

            let key = &self.keys[(start + i) % n];

            if !key.is_available() {
                continue;
            }

            attempts += 1;

            // Single CAS attempt per key — on failure, move to next.
            let cur = key.tpm_inflight.load(Ordering::Acquire);
            if cur + estimated_tokens > key.tpm_limit {
                continue;
            }
            match key.tpm_inflight.compare_exchange_weak(
                cur,
                cur + estimated_tokens,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(KeyLease {
                        inner: Arc::clone(key),
                        reserved_tokens: estimated_tokens,
                    });
                }
                Err(_) => {
                    std::hint::spin_loop();
                    continue;
                }
            }
        }

        None
    }

    /// Update key state based on a provider error response.
    ///
    /// Rate-limit cooldown and circuit-breaker cooldown are decoupled:
    /// - 429 → `cool_down_until` (rate-limit)
    /// - 5xx/network → `consecutive_failures` → `failure_cool_down_until` (circuit breaker)
    pub(crate) fn report_error(&self, lease: &KeyLease, err: &ApiError) {
        match err {
            ApiError::Unauthorized => {
                lease.inner.state.store(STATE_DEAD, Ordering::Release);
            }
            ApiError::RateLimited { retry_after } => {
                // Rate-limit cooldown — does NOT increment circuit breaker.
                let until = now_millis() + retry_after.as_millis() as u64;
                lease
                    .inner
                    .cool_down_until
                    .fetch_max(until, Ordering::AcqRel);
            }
            ApiError::Provider(_) => {
                // Circuit breaker: writes to failure_cool_down_until, NOT cool_down_until.
                let prev = lease
                    .inner
                    .consecutive_failures
                    .fetch_add(1, Ordering::Relaxed);
                if prev + 1 >= self.config.circuit_breaker_threshold {
                    let until =
                        now_millis() + self.config.circuit_breaker_cooldown.as_millis() as u64;
                    lease
                        .inner
                        .failure_cool_down_until
                        .store(until, Ordering::Release);
                    lease.inner.consecutive_failures.store(0, Ordering::Relaxed);
                }
            }
            ApiError::Cancelled => {
                // Upstream cancellation is not a key failure.
            }
            ApiError::Protocol(_) => {}
        }
    }

    /// Reset the circuit breaker counter on a successful response.
    pub(crate) fn report_success(&self, lease: &KeyLease) {
        lease.inner.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// Returns a snapshot of the current pool health for observability.
    pub fn status(&self) -> Vec<KeyStatus> {
        self.keys
            .iter()
            .map(|k| KeyStatus {
                label: k.label.clone(),
                available: k.is_available(),
                tpm_inflight: k.tpm_inflight.load(Ordering::Relaxed),
                tpm_limit: k.tpm_limit,
                cool_down_until: k.cool_down_until.load(Ordering::Relaxed),
                failure_cool_down_until: k.failure_cool_down_until.load(Ordering::Relaxed),
                consecutive_failures: k.consecutive_failures.load(Ordering::Relaxed),
            })
            .collect()
    }
}

/// A point-in-time snapshot of a single key's health, for observability.
#[derive(Debug, Clone)]
pub struct KeyStatus {
    /// Human-readable label of the key.
    pub label: String,
    /// Whether the key is currently available for selection.
    pub available: bool,
    /// Current in-flight TPM reservation.
    pub tpm_inflight: u32,
    /// Configured TPM limit.
    pub tpm_limit: u32,
    /// Rate-limit cooldown deadline (Unix millis). `0` = not cooling.
    pub cool_down_until: u64,
    /// Circuit-breaker cooldown deadline (Unix millis). `0` = not cooling.
    pub failure_cool_down_until: u64,
    /// Consecutive non-rate-limit failures since the last success.
    pub consecutive_failures: u32,
}
