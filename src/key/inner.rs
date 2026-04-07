//! Per-key atomic state, cooldown timestamps, and circuit breaker.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};

use crate::limiter::window::SlidingWindow;

/// Key is healthy and available for selection.
pub(crate) const STATE_HEALTHY: u8 = 0;
/// Key is permanently dead (e.g. HTTP 401). Never retry.
pub(crate) const STATE_DEAD: u8 = 1;

/// The internal state of a single API key.
///
/// All mutable state is stored in atomics for lock-free access from
/// concurrent requests. The [`SlidingWindow`] for RPM uses a per-key
/// `parking_lot::Mutex` — no cross-key contention.
pub(crate) struct KeyInner {
    /// The raw API key string sent as Authorization header.
    pub(crate) key: String,

    /// Human-readable label (e.g. `"openai-prod-1"`).
    pub(crate) label: String,

    /// Permanent health flag. [`STATE_DEAD`] = never retry.
    pub(crate) state: AtomicU8,

    // ── Rate-limit cooldown (429-driven) ───────────────────────
    /// Unix timestamp millis after which this key is healthy again.
    /// `0` means no active cooldown. Set by 429 responses.
    pub(crate) cool_down_until: AtomicU64,

    // ── Circuit breaker (error-driven, independent of rate-limit cooldown) ──
    /// Consecutive non-rate-limit failures (5xx, network, timeout).
    /// Reset to 0 on success. Triggers its own cooldown at threshold.
    pub(crate) consecutive_failures: AtomicU32,

    /// Separate cooldown timestamp for circuit-breaker trips.
    /// Decoupled from `cool_down_until` so a key recovering from a 429
    /// isn't immediately killed by a stale failure count.
    pub(crate) failure_cool_down_until: AtomicU64,

    /// In-flight token reservation (TPM pre-occupation).
    pub(crate) tpm_inflight: AtomicU32,

    /// Hard TPM cap for this key.
    pub(crate) tpm_limit: u32,

    /// Per-key RPM sliding window. `parking_lot::Mutex` — no cross-key contention.
    pub(crate) rpm_window: SlidingWindow,
}

impl KeyInner {
    /// Create a new `KeyInner` with the given configuration.
    pub(crate) fn new(key: String, label: String, tpm_limit: u32, rpm_limit: u32) -> Self {
        Self {
            key,
            label,
            state: AtomicU8::new(STATE_HEALTHY),
            cool_down_until: AtomicU64::new(0),
            consecutive_failures: AtomicU32::new(0),
            failure_cool_down_until: AtomicU64::new(0),
            tpm_inflight: AtomicU32::new(0),
            tpm_limit,
            rpm_window: SlidingWindow::new(rpm_limit, std::time::Duration::from_secs(60)),
        }
    }

    /// Returns `true` if the key is usable right now.
    ///
    /// Checks both rate-limit cooldown and circuit-breaker cooldown
    /// independently. Lazily clears expired cooldown timestamps.
    pub(crate) fn is_available(&self) -> bool {
        if self.state.load(Ordering::Acquire) == STATE_DEAD {
            return false;
        }

        let now = now_millis();

        // Check rate-limit cooldown.
        let rl_until = self.cool_down_until.load(Ordering::Acquire);
        if rl_until != 0 && now < rl_until {
            return false;
        }
        if rl_until != 0 {
            let _ = self.cool_down_until.compare_exchange(
                rl_until,
                0,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
        }

        // Check circuit-breaker cooldown (independent).
        let cb_until = self.failure_cool_down_until.load(Ordering::Acquire);
        if cb_until != 0 && now < cb_until {
            return false;
        }
        if cb_until != 0 {
            let _ = self.failure_cool_down_until.compare_exchange(
                cb_until,
                0,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
        }

        true
    }
}

/// Returns the current time as milliseconds since the Unix epoch.
#[inline]
pub(crate) fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
