---
title: Implementation Notes
description: Walk the crate module by module when you want the concrete execution path, data structures, and internal boundaries.
label: source walkthrough
release: v0.1.3
updated: Apr 2026
summary: Module layout, error model, and the core structs that enforce pool, limiter, and budget behavior.
---

# Implementation Notes

> Incorporates all review feedback: randomized key selection, per-key parking_lot Mutex,
> lazy cooldown evaluation, and zero zombie tasks.
>
> Note: this implementation document predates the canonical `Responses +
> Capability Layer` hybrid migration. It remains useful for gateway internals,
> but the public request/response examples no longer match the current API.

---

## Project Structure

```
omnillm/
├── Cargo.toml
├── src/
│   ├── lib.rs            # top-level re-exports and types
│   ├── error.rs
│   ├── key/
│   │   ├── mod.rs
│   │   ├── inner.rs      # KeyInner — atomic state, cooldown timestamps, circuit breaker
│   │   ├── lease.rs      # KeyLease — RAII quota guard
│   │   ├── pool.rs       # KeyPool — randomised selection, CAS reserve, circuit breaker
│   │   └── registry.rs   # PoolRegistry — Provider → Model → KeyPool routing
│   ├── limiter/
│   │   └── window.rs     # SlidingWindow — per-key sliding window using parking_lot Mutex
│   ├── budget/
│   │   └── tracker.rs    # BudgetTracker — fixed-point lock-free budget with two-phase settle
│   ├── pricing.rs        # Model price table, estimate / actual
│   ├── dispatcher/       # HTTP execution layer, stateless, key injected per-request
│   └── gateway.rs        # Gateway::call — main execution path with cancellation support
```

---

## Cargo.toml

```toml
[package]
name    = "omnillm"
version = "0.1.3"
edition = "2021"

[dependencies]
tokio        = { version = "1", features = ["full"] }
tokio-util   = "0.7"  # CancellationToken
reqwest      = { version = "0.12", features = ["json"] }
parking_lot  = "0.12"
rand         = "0.8"
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
thiserror    = "1"

[profile.release]
opt-level = 3
lto       = true
```

---

## src/error.rs

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("no healthy key with sufficient TPM capacity")]
    NoAvailableKey,

    #[error("budget limit exceeded")]
    BudgetExceeded,

    #[error("rate limited by local RPM window")]
    RateLimited,

    #[error("provider returned 401 — key is dead")]
    Unauthorized,

    #[error("request cancelled by upstream")]
    Cancelled,

    #[error("provider error: {0}")]
    Provider(String),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

/// Errors returned by the provider that affect key state.
#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    RateLimited { retry_after: std::time::Duration },
    ServerError(String),   // 5xx
    NetworkError(String),  // connection / DNS / timeout
    Cancelled,             // upstream cancellation
    Other(String),
}
```

---

## src/key/inner.rs

```rust
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::limiter::window::SlidingWindow;

/// Key state encoded as u8 for atomic storage.
pub const STATE_HEALTHY: u8 = 0;
pub const STATE_DEAD:    u8 = 1;
// NOTE: No STATE_COOLING — cooldown is lazy via timestamps.

pub struct KeyInner {
    /// The raw API key string sent as Authorization header.
    pub key: String,

    /// Human-readable label (e.g. "openai-prod-1").
    pub label: String,

    /// Permanent health flag. STATE_DEAD = never retry.
    pub state: std::sync::atomic::AtomicU8,

    /// --- Rate-limit cooldown (429-driven) ---
    /// Unix timestamp millis after which this key is healthy again.
    /// 0 means no active cooldown. Set by 429 responses.
    pub cool_down_until: AtomicU64,

    /// --- Circuit breaker (error-driven, independent of rate-limit cooldown) ---
    /// Consecutive non-rate-limit failures (5xx, network, timeout).
    /// Reset to 0 on success. Triggers its own cooldown at threshold.
    pub consecutive_failures: AtomicU32,

    /// Separate cooldown timestamp for circuit-breaker trips.
    /// Decoupled from cool_down_until so a key recovering from a 429
    /// isn't immediately killed by a stale failure count.
    pub failure_cool_down_until: AtomicU64,

    /// In-flight token reservation (TPM pre-occupation).
    pub tpm_inflight: AtomicU32,

    /// Hard TPM cap for this key.
    pub tpm_limit: u32,

    /// Per-key RPM sliding window. parking_lot Mutex — no cross-key contention.
    pub rpm_window: SlidingWindow,
}

impl KeyInner {
    pub fn new(key: String, label: String, tpm_limit: u32, rpm_limit: u32) -> Self {
        Self {
            key,
            label,
            state: std::sync::atomic::AtomicU8::new(STATE_HEALTHY),
            cool_down_until: AtomicU64::new(0),
            consecutive_failures: AtomicU32::new(0),
            failure_cool_down_until: AtomicU64::new(0),
            tpm_inflight: AtomicU32::new(0),
            tpm_limit,
            rpm_window: SlidingWindow::new(rpm_limit, std::time::Duration::from_secs(60)),
        }
    }

    /// Returns true if the key is usable right now.
    /// Checks both rate-limit cooldown and circuit-breaker cooldown independently.
    pub fn is_available(&self) -> bool {
        if self.state.load(Ordering::Acquire) == STATE_DEAD {
            return false;
        }

        let now = now_millis();

        // Check rate-limit cooldown
        let rl_until = self.cool_down_until.load(Ordering::Acquire);
        if rl_until != 0 && now < rl_until {
            return false;
        }
        if rl_until != 0 {
            let _ = self.cool_down_until.compare_exchange(
                rl_until, 0, Ordering::AcqRel, Ordering::Relaxed,
            );
        }

        // Check circuit-breaker cooldown (independent)
        let cb_until = self.failure_cool_down_until.load(Ordering::Acquire);
        if cb_until != 0 && now < cb_until {
            return false;
        }
        if cb_until != 0 {
            let _ = self.failure_cool_down_until.compare_exchange(
                cb_until, 0, Ordering::AcqRel, Ordering::Relaxed,
            );
        }

        true
    }
}

#[inline]
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
```

---

## src/key/lease.rs

```rust
use std::sync::Arc;
use std::sync::atomic::Ordering;
use super::inner::KeyInner;

/// RAII quota guard. Dropping this unconditionally returns the reserved
/// TPM tokens — regardless of success, error, panic, or async cancellation.
pub struct KeyLease {
    pub inner: Arc<KeyInner>,
    pub reserved_tokens: u32,
}

impl Drop for KeyLease {
    fn drop(&mut self) {
        self.inner
            .tpm_inflight
            .fetch_sub(self.reserved_tokens, Ordering::Release);
    }
}
```

---

## src/key/pool.rs

```rust
use std::sync::Arc;
use std::sync::atomic::Ordering;
use rand::Rng;

use super::inner::{KeyInner, STATE_DEAD, now_millis};
use super::lease::KeyLease;
use crate::error::ApiError;

/// Maximum number of keys to attempt CAS on before giving up.
/// Bounds worst-case CPU time under extreme contention.
const MAX_CAS_ATTEMPTS: usize = 5;

/// Consecutive non-rate-limit failures before circuit breaker trips.
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;

/// How long a circuit-broken key stays out of rotation.
const CIRCUIT_BREAKER_COOLDOWN_MS: u64 = 30_000; // 30 seconds

pub struct KeyPool {
    keys: Vec<Arc<KeyInner>>,
}

impl KeyPool {
    pub fn new(keys: Vec<Arc<KeyInner>>) -> Self {
        Self { keys }
    }

    /// Select a key and atomically reserve `estimated_tokens` of TPM quota.
    ///
    /// Strategy: randomized start index + first-fit, capped at MAX_CAS_ATTEMPTS.
    /// - Avoids thundering herd: concurrent requests start from different indices.
    /// - On CAS failure, skips to next key instead of spinning.
    /// - Hard cap prevents long-tail CPU spin under extreme contention.
    /// - O(1) amortized when keys have sufficient capacity.
    pub fn acquire(&self, estimated_tokens: u32) -> Option<KeyLease> {
        if self.keys.is_empty() {
            return None;
        }

        let n = self.keys.len();
        let start = rand::thread_rng().gen_range(0..n);
        let mut attempts = 0;

        for i in 0..n {
            if attempts >= MAX_CAS_ATTEMPTS {
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
    /// Rate-limit cooldown and circuit-breaker cooldown are decoupled:
    /// - 429 → cool_down_until (rate-limit)
    /// - 5xx/network → consecutive_failures → failure_cool_down_until (circuit breaker)
    pub fn report_error(&self, lease: &KeyLease, err: &ApiError) {
        match err {
            ApiError::Unauthorized => {
                lease.inner.state.store(STATE_DEAD, Ordering::Release);
            }
            ApiError::RateLimited { retry_after } => {
                // Rate-limit cooldown — does NOT increment circuit breaker.
                let until = now_millis() + retry_after.as_millis() as u64;
                lease.inner.cool_down_until.fetch_max(until, Ordering::AcqRel);
            }
            ApiError::ServerError(_) | ApiError::NetworkError(_) => {
                // Circuit breaker: writes to failure_cool_down_until, NOT cool_down_until.
                let prev = lease.inner.consecutive_failures
                    .fetch_add(1, Ordering::Relaxed);
                if prev + 1 >= CIRCUIT_BREAKER_THRESHOLD {
                    let until = now_millis() + CIRCUIT_BREAKER_COOLDOWN_MS;
                    lease.inner.failure_cool_down_until.store(until, Ordering::Release);
                    lease.inner.consecutive_failures.store(0, Ordering::Relaxed);
                }
            }
            ApiError::Cancelled => {
                // Upstream cancellation is not a key failure.
            }
            ApiError::Other(_) => {}
        }
    }

    /// Reset circuit breaker on successful response.
    pub fn report_success(&self, lease: &KeyLease) {
        lease.inner.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// Returns a snapshot of current pool health for observability.
    pub fn status(&self) -> Vec<KeyStatus> {
        self.keys.iter().map(|k| KeyStatus {
            label: k.label.clone(),
            available: k.is_available(),
            tpm_inflight: k.tpm_inflight.load(Ordering::Relaxed),
            tpm_limit: k.tpm_limit,
            cool_down_until: k.cool_down_until.load(Ordering::Relaxed),
            failure_cool_down_until: k.failure_cool_down_until.load(Ordering::Relaxed),
            consecutive_failures: k.consecutive_failures.load(Ordering::Relaxed),
        }).collect()
    }
}

#[derive(Debug)]
pub struct KeyStatus {
    pub label: String,
    pub available: bool,
    pub tpm_inflight: u32,
    pub tpm_limit: u32,
    pub cool_down_until: u64,
    pub failure_cool_down_until: u64,
    pub consecutive_failures: u32,
}
```

---

## src/limiter/window.rs

```rust
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use parking_lot::Mutex;

/// Sliding window rate limiter.
///
/// Uses parking_lot::Mutex (3-5x faster than std::sync::Mutex on uncontended
/// paths). Per-key instantiation means there is zero cross-key lock contention.
///
/// Why not a time-wheel / AtomicU32 bucket array?
/// - Bucket arrays require a separate "clear expired bucket" step that itself
///   needs synchronisation.
/// - parking_lot Mutex held for ~100ns (a few pop_fronts) has negligible
///   impact compared to the LLM network round-trip (100ms–10s).
/// - Sliding window semantics match OpenAI's actual RPM window exactly.
///   Fixed-bucket schemes admit bursts at bucket boundaries.
pub struct SlidingWindow {
    limit: u32,
    window: Duration,
    timestamps: Mutex<VecDeque<Instant>>,
}

impl SlidingWindow {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            timestamps: Mutex::new(VecDeque::with_capacity(limit as usize)),
        }
    }

    /// Attempt to record one request. Returns false if the window is full.
    pub fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;
        let mut ts = self.timestamps.lock();

        // Evict expired entries from the front.
        while ts.front().map_or(false, |t| *t < cutoff) {
            ts.pop_front();
        }

        if ts.len() < self.limit as usize {
            ts.push_back(now);
            true
        } else {
            false
        }
    }

    /// How many slots remain in the current window (for observability).
    pub fn remaining(&self) -> u32 {
        let now = Instant::now();
        let cutoff = now - self.window;
        let ts = self.timestamps.lock();
        let active = ts.iter().filter(|t| **t >= cutoff).count();
        self.limit.saturating_sub(active as u32)
    }
}
```

---

## src/budget/tracker.rs

```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// Fixed-point cost representation.
/// 1 USD = 1_000_000 micro-dollars.
/// Using u64 avoids float precision loss and enables lock-free CAS.
pub type MicroDollar = u64;

pub struct BudgetTracker {
    limit: MicroDollar,
    used: AtomicU64,
}

impl BudgetTracker {
    pub fn new(limit_usd: f64) -> Self {
        Self {
            limit: usd_to_micro(limit_usd),
            used: AtomicU64::new(0),
        }
    }

    /// Pre-occupy `estimated` micro-dollars.
    /// Returns false without modifying state if the budget would be exceeded.
    pub fn try_reserve(&self, estimated: MicroDollar) -> bool {
        loop {
            let cur = self.used.load(Ordering::Acquire);
            if cur + estimated > self.limit {
                return false;
            }
            if self.used
                .compare_exchange_weak(
                    cur,
                    cur + estimated,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Settle the difference between pre-estimated and actual cost.
    /// Called after the provider response arrives.
    /// `actual = 0` on error (full refund of the reservation).
    pub fn settle(&self, estimated: MicroDollar, actual: MicroDollar) {
        if actual > estimated {
            self.used.fetch_add(actual - estimated, Ordering::Relaxed);
        } else {
            self.used.fetch_sub(estimated - actual, Ordering::Relaxed);
        }
    }

    pub fn used_usd(&self) -> f64 {
        micro_to_usd(self.used.load(Ordering::Relaxed))
    }

    pub fn limit_usd(&self) -> f64 {
        micro_to_usd(self.limit)
    }

    pub fn remaining_usd(&self) -> f64 {
        let used = self.used.load(Ordering::Relaxed);
        micro_to_usd(self.limit.saturating_sub(used))
    }
}

#[inline] pub fn usd_to_micro(usd: f64) -> MicroDollar { (usd * 1_000_000.0) as u64 }
#[inline] pub fn micro_to_usd(micro: MicroDollar) -> f64 { micro as f64 / 1_000_000.0 }
```

---

## src/pricing.rs

```rust
use crate::budget::{tracker::{MicroDollar, usd_to_micro}};

pub struct ModelPricing {
    /// Cost per 1k input tokens in micro-dollars.
    pub input_per_1k: MicroDollar,
    /// Cost per 1k output tokens in micro-dollars.
    pub output_per_1k: MicroDollar,
}

/// Token usage reported in the provider response.
#[derive(Debug, Default)]
pub struct TokenUsage {
    pub prompt_tokens:     u32,
    pub completion_tokens: u32,
}

impl TokenUsage {
    pub fn total(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Estimate cost from token count before the request is sent.
/// Uses total tokens with input pricing as a conservative upper bound.
pub fn estimate(tokens: u32, model: &str) -> MicroDollar {
    let p = pricing(model);
    (tokens as u64 * p.input_per_1k) / 1000
}

/// Compute actual cost from the response's usage report.
pub fn actual(usage: &TokenUsage, model: &str) -> MicroDollar {
    let p = pricing(model);
    let input  = (usage.prompt_tokens     as u64 * p.input_per_1k)  / 1000;
    let output = (usage.completion_tokens as u64 * p.output_per_1k) / 1000;
    input + output
}

fn pricing(model: &str) -> ModelPricing {
    // Prices as of mid-2025; update as providers change rates.
    match model {
        m if m.starts_with("gpt-4o-mini") => ModelPricing {
            input_per_1k:  usd_to_micro(0.000150),
            output_per_1k: usd_to_micro(0.000600),
        },
        m if m.starts_with("gpt-4o") => ModelPricing {
            input_per_1k:  usd_to_micro(0.005),
            output_per_1k: usd_to_micro(0.015),
        },
        m if m.starts_with("claude-3-5-sonnet") => ModelPricing {
            input_per_1k:  usd_to_micro(0.003),
            output_per_1k: usd_to_micro(0.015),
        },
        m if m.starts_with("claude-3-haiku") => ModelPricing {
            input_per_1k:  usd_to_micro(0.00025),
            output_per_1k: usd_to_micro(0.00125),
        },
        _ => ModelPricing {
            // Unknown model: charge at GPT-4o rate (conservative).
            input_per_1k:  usd_to_micro(0.005),
            output_per_1k: usd_to_micro(0.015),
        },
    }
}
```

---

## src/dispatcher.rs

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, GatewayError};
use crate::key::lease::KeyLease;
use crate::pricing::TokenUsage;

#[derive(Debug, Serialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl LlmRequest {
    /// Conservative token estimate before sending.
    /// Real count is only available in the response.
    pub fn estimated_tokens(&self) -> u32 {
        // ~4 chars per token; count message chars as a proxy.
        let chars: usize = self.messages.iter()
            .map(|m| m.content.len())
            .sum();
        (chars / 4).max(1) as u32 + self.max_tokens.unwrap_or(1024)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role:    String,
    pub content: String,
}

#[derive(Debug)]
pub struct LlmResponse {
    pub content: String,
    pub usage:   TokenUsage,
    pub model:   String,
}

/// Stateless HTTP executor.
///
/// Holds a single reqwest::Client so the underlying TCP/TLS connection pool
/// is shared across all requests and all keys. API keys are injected per-request
/// via the Authorization header — the client itself never holds credentials.
pub struct Dispatcher {
    client:       Client,
    provider_url: String,
}

impl Dispatcher {
    pub fn new(provider_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("failed to build reqwest client"),
            provider_url: provider_url.into(),
        }
    }

    /// Send the request using the key from `lease`.
    /// Translates provider HTTP errors into typed `ApiError` for pool reporting.
    pub async fn call(
        &self,
        lease: &KeyLease,
        req: &LlmRequest,
    ) -> Result<LlmResponse, ApiError> {
        let resp = self.client
            .post(&self.provider_url)
            .header("Authorization", format!("Bearer {}", lease.inner.key))
            .header("Content-Type", "application/json")
            .json(req)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    ApiError::NetworkError(e.to_string())
                } else {
                    ApiError::Other(e.to_string())
                }
            })?;

        match resp.status().as_u16() {
            200 => {
                let body: serde_json::Value = resp.json().await
                    .map_err(|e| ApiError::NetworkError(e.to_string()))?;
                Ok(parse_response(body))
            }
            401 | 403 => Err(ApiError::Unauthorized),
            429 => {
                let retry_after = resp.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(std::time::Duration::from_secs)
                    .unwrap_or(std::time::Duration::from_secs(60));
                Err(ApiError::RateLimited { retry_after })
            }
            status @ 500..=599 => {
                let text = resp.text().await.unwrap_or_default();
                Err(ApiError::ServerError(format!("HTTP {status}: {text}")))
            }
            status => {
                let text = resp.text().await.unwrap_or_default();
                Err(ApiError::Other(format!("HTTP {status}: {text}")))
            }
        }
    }
}

fn parse_response(body: serde_json::Value) -> LlmResponse {
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let usage = TokenUsage {
        prompt_tokens:     body["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        completion_tokens: body["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
    };

    let model = body["model"].as_str().unwrap_or("unknown").to_string();

    LlmResponse { content, usage, model }
}
```

---

## src/gateway.rs

```rust
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::budget::tracker::BudgetTracker;
use crate::dispatcher::{Dispatcher, LlmRequest, LlmResponse};
use crate::error::{ApiError, GatewayError};
use crate::key::pool::KeyPool;
use crate::pricing;

pub struct Gateway {
    pool:       Arc<KeyPool>,
    budget:     Arc<BudgetTracker>,
    dispatcher: Arc<Dispatcher>,
}

impl Gateway {
    pub fn new(
        pool:       Arc<KeyPool>,
        budget:     Arc<BudgetTracker>,
        dispatcher: Arc<Dispatcher>,
    ) -> Self {
        Self { pool, budget, dispatcher }
    }

    /// Main execution path. Order of operations is load-bearing:
    ///
    /// 1. Acquire key   — if this fails, no budget is consumed.
    /// 2. Reserve budget — if this fails, lease drops and TPM is returned.
    /// 3. RPM check      — if this fails, budget is refunded, lease drops.
    /// 4. Dispatch       — actual HTTP call with cancellation propagation.
    /// 5. Accounting     — settle budget, update key state on error.
    ///    lease drops here — TPM returned unconditionally.
    pub async fn call(
        &self,
        req: LlmRequest,
        cancel: CancellationToken,
    ) -> Result<LlmResponse, GatewayError> {
        let est_tokens = req.estimated_tokens();
        let est_cost   = pricing::estimate(est_tokens, &req.model);

        // ── 1. Acquire key (select + CAS reserve, atomic) ─────────────────
        let lease = self.pool
            .acquire(est_tokens)
            .ok_or(GatewayError::NoAvailableKey)?;

        // ── 2. Pre-occupy budget ───────────────────────────────────────────
        if !self.budget.try_reserve(est_cost) {
            return Err(GatewayError::BudgetExceeded);
        }

        // ── 3. Local RPM check ─────────────────────────────────────────────
        if !lease.inner.rpm_window.try_acquire() {
            self.budget.settle(est_cost, 0);
            return Err(GatewayError::RateLimited);
        }

        // ── 4. Dispatch with cancellation propagation ──────────────────────
        //    If upstream cancels, we abort the HTTP request — not just drop
        //    the future. Prevents phantom inflight: the provider stops
        //    processing, and our TPM counter reflects reality.
        let result = tokio::select! {
            res = self.dispatcher.call(&lease, &req) => res,
            _ = cancel.cancelled() => Err(ApiError::Cancelled),
        };

        // ── 5. Accounting ──────────────────────────────────────────────────
        match &result {
            Ok(resp) => {
                let actual = pricing::actual(&resp.usage, &req.model);
                self.budget.settle(est_cost, actual);
                self.pool.report_success(&lease);
            }
            Err(ApiError::Cancelled) => {
                self.budget.settle(est_cost, 0);
                // Not a key failure — don't touch circuit breaker
            }
            Err(api_err) => {
                self.budget.settle(est_cost, 0);
                self.pool.report_error(&lease, api_err);
            }
        }

        // lease drops here → TPM returned ✓
        result.map_err(|e| match e {
            ApiError::Unauthorized                 => GatewayError::Unauthorized,
            ApiError::RateLimited { .. }           => GatewayError::RateLimited,
            ApiError::Cancelled                    => GatewayError::Cancelled,
            ApiError::ServerError(msg)             => GatewayError::Provider(msg),
            ApiError::NetworkError(msg)            => GatewayError::Provider(msg),
            ApiError::Other(msg)                   => GatewayError::Provider(msg),
        })
    }

    pub fn pool_status(&self) -> Vec<crate::key::pool::KeyStatus> {
        self.pool.status()
    }

    pub fn budget_remaining_usd(&self) -> f64 {
        self.budget.remaining_usd()
    }
}
```

---

## src/lib.rs

```rust
pub mod error;
pub mod key {
    pub mod inner;
    pub mod lease;
    pub mod pool;
    pub mod registry;
}
pub mod limiter {
    pub mod window;
}
pub mod budget {
    pub mod tracker;
}
pub mod pricing;
pub mod dispatcher;
pub mod gateway;

// Re-exports for convenience.
pub use gateway::Gateway;
pub use key::{inner::KeyInner, pool::KeyPool, registry::PoolRegistry};
pub use budget::tracker::BudgetTracker;
pub use dispatcher::Dispatcher;
pub use error::GatewayError;
```

---

## Usage Example

```rust
// main.rs
use std::sync::Arc;
use llm_gateway::{Gateway, KeyInner, KeyPool, BudgetTracker, Dispatcher};
use llm_gateway::dispatcher::{LlmRequest, Message};

#[tokio::main]
async fn main() {
    // Build key pool — multiple keys, different limits.
    let keys = vec![
        Arc::new(KeyInner::new(
            "sk-prod-key-1".into(),
            "openai-prod-1".into(),
            /*tpm_limit=*/ 90_000,
            /*rpm_limit=*/ 500,
        )),
        Arc::new(KeyInner::new(
            "sk-prod-key-2".into(),
            "openai-prod-2".into(),
            /*tpm_limit=*/ 90_000,
            /*rpm_limit=*/ 500,
        )),
        Arc::new(KeyInner::new(
            "sk-fallback-key".into(),
            "openai-fallback".into(),
            /*tpm_limit=*/ 40_000,
            /*rpm_limit=*/ 60,
        )),
    ];

    let gateway = Arc::new(Gateway::new(
        Arc::new(KeyPool::new(keys)),
        Arc::new(BudgetTracker::new(/*limit_usd=*/ 50.0)),
        Arc::new(Dispatcher::new("https://api.openai.com/v1/chat/completions")),
    ));

    // Simulate concurrent calls.
    let mut handles = Vec::new();
    for i in 0..10 {
        let gw = Arc::clone(&gateway);
        handles.push(tokio::spawn(async move {
            let req = LlmRequest {
                model:      "gpt-4o-mini".into(),
                messages:   vec![Message {
                    role:    "user".into(),
                    content: format!("Request number {i}"),
                }],
                max_tokens: Some(256),
            };
            match gw.call(req).await {
                Ok(resp)  => println!("[{i}] ok — {} tokens used", resp.usage.total()),
                Err(e)    => println!("[{i}] err — {e}"),
            }
        }));
    }
    for h in handles { h.await.unwrap(); }

    // Observability.
    println!("\nBudget remaining: ${:.4}", gateway.budget_remaining_usd());
    for s in gateway.pool_status() {
        println!(
            "Key {:20} available={} inflight={}/{}",
            s.label, s.available, s.tpm_inflight, s.tpm_limit
        );
    }
}
```

---

## Design Decision Log

| Decision | Choice | Rejected | Reason |
|---|---|---|---|
| TPM quota return | RAII `Drop` on `KeyLease` | Explicit `release()` | Drop survives async cancel, panic, and `?` propagation |
| Key selection order | Random start index + first-fit | `min_by_key` (least loaded) | Distributes CAS collisions; O(1) amortized; no thundering herd |
| CAS failure handling | Skip to next key, cap at MAX_CAS_ATTEMPTS | Spin-retry same key | Bounded CPU time; unbounded scan degenerates under contention |
| RPM check ordering | After TPM reserve (lock-free path) | Inside CAS loop | Merging requires Mutex inside CAS; transient saturation is microsecond-scale |
| Cooldown mechanism | Lazy `cool_down_until: AtomicU64` | `tokio::spawn` timer | Zero zombie tasks; no heap allocation per 429 |
| Cooldown decoupling | Separate `cool_down_until` + `failure_cool_down_until` | Single timestamp | Prevents 429 recovery + stale failure count = false breaker trip |
| Cooldown write | `fetch_max` | `store` | Prevents a shorter cooldown from overwriting a longer one |
| Error resilience | Circuit breaker (`consecutive_failures` → `failure_cool_down_until`) | Ignore 5xx/network | Bad key absorbs traffic indefinitely |
| Cancellation | `tokio::select!` + `CancellationToken` | Drop-only | Prevents phantom inflight; provider stops processing |
| Cost representation | `u64` micro-dollar | `f32`/`f64` | No precision loss; enables atomic CAS |
| Rate limit algorithm | Sliding window | GCRA (governor) | Matches OpenAI's actual RPM window semantics; allows legal bursts |
| Mutex choice | `parking_lot::Mutex` | `std::sync::Mutex` | 3-5x faster on uncontended paths; no OS syscall on fast path |
| Lock scope | Per-key `SlidingWindow` | Global rate limiter | Zero cross-key contention by construction |
| Budget phase | Pre-reserve + `settle()` | Post-record only | Pre-reserve prevents concurrent overspend |
| Key injection | Header per-request | `provider.with_api_key()` | Single `Client` → shared TCP/TLS connection pool |
| Error taxonomy | `ApiError` (pool) vs `GatewayError` (caller) | Single error type | Pool errors drive state transitions; caller errors drive retry logic |
