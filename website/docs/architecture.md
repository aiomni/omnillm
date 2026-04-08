---
title: Architecture Notes
description: Read the key-pool acquisition model, lease lifecycle, and budget tracker design before diving into the source.
label: system design
release: v0.1.0
updated: Apr 2026
summary: Random-start acquisition, explicit cooldown semantics, and fixed-point budget settlement across the runtime.
---

# Architecture Notes

> Production-grade Rust scheduling kernel for LLM API access.
> Handles multi-key load balancing, per-key rate limiting, cost tracking,
> and error-driven state management — with all critical paths concurrency-safe.
>
> Note: the public request/response model has since moved to a canonical
> `Responses + Capability Layer` hybrid. The concurrency, pool, and budget
> architecture below still applies, but some request/response examples are
> from the pre-migration Chat Completions model.

---

## Design Principles

**Keys are leases, not config.** A key is acquired before use and released after — unconditionally, via `Drop`. There is no code path where a key can be consumed without being returned.

**Accounting is a first-class citizen.** Every request pre-occupies quota (tokens, budget). On response, actual usage settles the pre-occupation. Errors trigger key state transitions immediately.

**Algorithm choice follows provider semantics.** OpenAI's RPM is a sliding window — not a token bucket, not GCRA. The limiter models the actual provider behavior, not a convenient approximation.

> ⚠️ **select + reserve must be atomic.** Any system that separates key selection from quota reservation has a TOCTOU window. This design merges both into a single CAS loop inside `KeyPool::acquire`.
>
> ⚠️ **Precision and lock-freedom are not opposites.** Per-key `parking_lot::Mutex` has no cross-key contention, microsecond hold times, and near-zero cost when uncontended. Timestamp-based cooldown is both precise (millisecond granularity) and fully lock-free. There is no trade-off here — you get both.

---

## Module Layout

```
omnillm/
├── key/
│   ├── pool.rs       # KeyPool — per-model pool, acquire, error reporting, circuit breaker
│   ├── lease.rs      # KeyLease — RAII quota lease
│   └── registry.rs   # PoolRegistry — Provider → Model → KeyPool routing
├── limiter/
│   └── window.rs     # SlidingWindow — RPM and TPM rate control
├── budget/
│   └── tracker.rs    # BudgetTracker — fixed-point cost tracking
├── scheduler/
│   └── mod.rs        # Key selection strategy
├── dispatcher/
│   └── mod.rs        # HTTP execution, key header injection, retry/fallback
└── gateway/
    └── mod.rs        # Public entrypoint — Gateway::call
```

### Request flow

Every call follows this path — no exceptions.

```
LlmRequest
  → Gateway::call()
      → KeyPool::acquire()           // select + CAS reserve → KeyLease
      → BudgetTracker::try_reserve() // pre-occupy estimated cost
      → SlidingWindow::try_acquire() // RPM check
      → Dispatcher::call()           // HTTP, key injected via header
      → accounting()                 // settle budget, update key state
      → drop(lease)                  // TPM quota returned unconditionally
```

---

## KeyLease — RAII Lease

The central insight of this design. A `KeyLease` holds a reservation of TPM quota against a specific `KeyInner`. When it drops — whether the call succeeded, panicked, or was cancelled — the quota is returned via `fetch_sub`. There is no way to forget.

```rust
// key/lease.rs

pub struct KeyLease {
    inner: Arc<KeyInner>,
    reserved_tokens: u32,
}

struct KeyInner {
    key: String,
    provider: ProviderId,
    tpm_inflight: AtomicU32,
    tpm_limit: u32,
    // STATE_HEALTHY = 0 | STATE_DEAD = 1
    state: AtomicU8,
    // --- Cooldown (rate-limit driven) ---
    // Unix timestamp millis; 0 = not cooling down.
    // Set by 429 responses. Checked lazily in acquire().
    cool_down_until: AtomicU64,
    // --- Circuit breaker (error driven, independent of cooldown) ---
    // Consecutive non-rate-limit failures (5xx, network, timeout).
    // Reset to 0 on any success. Triggers its own cooldown at threshold.
    // Decoupled from cool_down_until so that a key recovering from a 429
    // cooldown isn't immediately killed by a stale failure count.
    consecutive_failures: AtomicU32,
    failure_cool_down_until: AtomicU64,
}

impl Drop for KeyLease {
    fn drop(&mut self) {
        // Unconditional — runs on success, error, panic, or async cancel
        self.inner
            .tpm_inflight
            .fetch_sub(self.reserved_tokens, Ordering::Release);
    }
}
```

> **Why not a guard pattern with explicit release?** Because explicit release is forgettable. Any early-return, `?` propagation, or future cancellation would skip it. `Drop` is the only guarantee that survives async cancellation in Tokio.

---

## KeyPool — Acquire and Error Reporting

The pool uses a **random-start first-fit** strategy: each request begins scanning from a random index, wraps around, and takes the first healthy key with available capacity. This avoids the thundering herd problem of `min_by_key` — where N concurrent requests all see the same "least-loaded" key and pile onto the same CAS — and naturally distributes requests across keys with O(1) amortised cost.

The scan is **capped at `MAX_CAS_ATTEMPTS`** (default 5). Under extreme contention with very few keys, unbounded scanning degenerates into a long-tail spin. Capping attempts converts this into a fast, bounded failure — the caller gets `None` and can retry at a higher level (e.g. `FallbackScheduler`) rather than burning CPU.

```rust
// key/pool.rs — acquire

const MAX_CAS_ATTEMPTS: usize = 5;

pub fn acquire(&self, estimated_tokens: u32) -> Option<KeyLease> {
    let now_ms = current_millis();
    let start = rand::random::<usize>() % self.keys.len();
    let candidates = self.keys[start..]
        .iter()
        .chain(self.keys[..start].iter())
        .filter(|k| k.state.load(Ordering::Acquire) == STATE_HEALTHY)
        .filter(|k| {
            let rl = k.cool_down_until.load(Ordering::Acquire);
            let cb = k.failure_cool_down_until.load(Ordering::Acquire);
            (rl == 0 || now_ms >= rl) && (cb == 0 || now_ms >= cb)
        });

    let mut attempts = 0;
    for key in candidates {
        if attempts >= MAX_CAS_ATTEMPTS {
            break; // hard cap — avoid long-tail spin
        }
        attempts += 1;

        let cur = key.tpm_inflight.load(Ordering::Relaxed);
        if cur + estimated_tokens > key.tpm_limit {
            continue;
        }

        // Single CAS attempt per key — on failure, skip to next.
        let result = key.tpm_inflight.compare_exchange_weak(
            cur,
            cur + estimated_tokens,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );
        if result.is_ok() {
            return Some(KeyLease {
                inner: Arc::clone(key),
                reserved_tokens: estimated_tokens,
            });
        }
        std::hint::spin_loop();
    }
    None
}
```

```rust
// key/pool.rs — error reporting + circuit breaker

const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;
const CIRCUIT_BREAKER_COOLDOWN_MS: u64 = 30_000; // 30 seconds

pub fn report_error(&self, lease: &KeyLease, err: &ApiError) {
    match err {
        ApiError::Unauthorized => {
            lease.inner.state.store(STATE_DEAD, Ordering::Release);
        }
        ApiError::RateLimited { retry_after } => {
            // Rate-limit cooldown — independent of circuit breaker.
            // A 429 is not an "error"; it's the provider saying "slow down".
            // Does NOT increment consecutive_failures.
            let until = current_millis() + retry_after.as_millis() as u64;
            lease.inner.cool_down_until.fetch_max(until, Ordering::AcqRel);
        }
        ApiError::ServerError(_) | ApiError::NetworkError(_) => {
            // Circuit breaker: writes to failure_cool_down_until, NOT cool_down_until.
            // This decoupling prevents the scenario where a key recovers from a
            // 429 cooldown, gets one 5xx (which would be failure #5 from before
            // the cooldown), and is immediately killed again.
            let prev = lease.inner.consecutive_failures
                .fetch_add(1, Ordering::Relaxed);
            if prev + 1 >= CIRCUIT_BREAKER_THRESHOLD {
                let until = current_millis() + CIRCUIT_BREAKER_COOLDOWN_MS;
                lease.inner.failure_cool_down_until.store(until, Ordering::Release);
                lease.inner.consecutive_failures.store(0, Ordering::Relaxed);
            }
        }
        _ => {}
    }
}

pub fn report_success(&self, lease: &KeyLease) {
    lease.inner.consecutive_failures.store(0, Ordering::Relaxed);
}
```

> **Why timestamps instead of `tokio::spawn`?** The spawn-based approach creates a zombie timer problem: the spawned task holds an `Arc<KeyInner>` and a timer handle that outlives the request. If the pool is dropped or keys are reconfigured, these timers keep running against stale state. The timestamp approach is checked lazily during `acquire()` — zero async overhead, zero memory overhead, and the key is automatically eligible again once `current_millis()` passes the deadline.

> **Decoupled cooldowns.** Rate-limit cooldown (`cool_down_until`) and circuit-breaker cooldown (`failure_cool_down_until`) are separate timestamps, checked independently in `acquire()`. This prevents a subtle failure mode: if a key accumulates 4 failures, then gets a 429 and cools down for 60s, and the first request after recovery hits a 5xx, a shared counter would immediately trip the breaker (failure #5) even though the errors were spread over a minute. Separate timestamps, separate counters, separate semantics.

> ⚠️ **CAS contention.** With random-start + skip-on-fail + `MAX_CAS_ATTEMPTS` cap, CAS contention is bounded both probabilistically and absolutely. Each acquire makes at most 5 CAS attempts, then returns `None` for the caller to handle at a higher level. If you have >1000 concurrent callers with very few keys, consider sharding the pool.

---

## SlidingWindow — RPM and TPM Rate Control

Each key carries two `SlidingWindow` instances — one for RPM, one for TPM. The sliding window accurately models OpenAI's actual rate limit behavior, unlike GCRA (which is more conservative) or fixed-window (which allows boundary bursts).

```rust
// limiter/window.rs

pub struct SlidingWindow {
    window: Duration,
    limit: u32,
    timestamps: Mutex<VecDeque<Instant>>,
}

impl SlidingWindow {
    pub fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let mut ts = self.timestamps.lock().unwrap();

        // Evict entries that have fallen outside the window
        let cutoff = now - self.window;
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
}
```

> **Why not `governor`?** The `governor` crate uses GCRA, which enforces uniform inter-arrival spacing. OpenAI's RPM allows bursts within a window — 60 requests can arrive in the first second of a minute window, which GCRA would reject. The sliding window here allows that burst naturally.

> **Mutex choice.** The `Mutex` hold time here is microsecond-scale (a `pop_front` loop), so the real concern is not the lock itself but the implementation. Use `parking_lot::Mutex` instead of `std::sync::Mutex` — it is ~3-5× faster under contention, does not poison on panic, and in an async context avoids blocking the Tokio worker thread for meaningful durations. Since each `SlidingWindow` is per-key, there is no cross-key lock contention — multiple keys are fully independent.

> ⚠️ **Memory bound.** `VecDeque<Instant>` holds at most `limit` entries. For RPM=10000, that is ~80KB per key. Acceptable for tens of keys; if you have thousands of keys, consider a fixed-capacity ring buffer instead.

---

## BudgetTracker — Fixed-Point, Lock-Free

Costs are stored as `u64` micro-dollars (1 USD = 1,000,000 units). This avoids floating-point precision loss and enables atomic CAS operations. A two-phase settle corrects the delta between pre-estimated and actual usage.

```rust
// budget/tracker.rs

/// 1 USD = 1_000_000 micro-dollars
pub type MicroDollar = u64;

pub struct BudgetTracker {
    limit: MicroDollar,
    used: AtomicU64,
}

impl BudgetTracker {
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

    /// Called after response: correct estimated → actual delta
    pub fn settle(&self, estimated: MicroDollar, actual: MicroDollar) {
        if actual > estimated {
            self.used.fetch_add(actual - estimated, Ordering::Relaxed);
        } else {
            self.used.fetch_sub(estimated - actual, Ordering::Relaxed);
        }
    }
}
```

---

## Gateway::call — The Main Path

The gateway wires all components together. The order of operations is significant: acquire key before budget (so a failed acquire doesn't consume budget), settle before lease drop (so accounting runs with the key still reserved).

```rust
// gateway/mod.rs

pub async fn call(&self, req: LlmRequest, cancel: CancellationToken) -> Result<LlmResponse> {
    let est_tokens = req.estimated_tokens();
    let est_cost   = pricing::estimate(est_tokens, &req.model);

    // 1. Acquire key (select + CAS reserve in one step)
    let lease = self.pool.acquire(est_tokens)
        .ok_or(Error::NoAvailableKey)?;

    // 2. Pre-occupy budget
    if !self.budget.try_reserve(est_cost) {
        return Err(Error::BudgetExceeded);
    }

    // 3. Rate limit check
    if !lease.inner.rpm_window.try_acquire() {
        self.budget.settle(est_cost, 0);
        return Err(Error::RateLimited);
    }

    // 4. Execute with cancellation propagation.
    //    If upstream cancels, we abort the HTTP request — not just drop
    //    the future. This prevents phantom inflight: the provider stops
    //    processing, and our TPM counter reflects reality.
    let result = tokio::select! {
        res = self.dispatcher.call(&lease, &req) => res,
        _ = cancel.cancelled() => Err(ApiError::Cancelled),
    };

    // 5. Accounting — always runs before lease drops
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
        Err(e) => {
            self.budget.settle(est_cost, 0);
            self.pool.report_error(&lease, e);
        }
    }

    // lease drops here → TPM returned ✓
    result
}
```

> **Dispatcher injects the key as a header, never clones the client.** `Dispatcher` holds a single `reqwest::Client` (shared connection pool). The API key is injected per-request via `Authorization: Bearer {lease.inner.key}`. The provider is stateless.

> ⚠️ **RPM tradeoff.** RPM is checked *after* TPM reservation. Under burst load (e.g. 1000 concurrent requests), requests that pass TPM but fail RPM briefly inflate `tpm_inflight` before the lease drops. This transient saturation may cause other requests to see false "full" states and return `None`. The window is microsecond-scale and self-healing, but it can amplify tail latency under extreme bursts. We accept this tradeoff to keep the acquire path lock-free — merging RPM into the CAS loop would require holding a Mutex inside a CAS, which is worse.

### Dispatcher — Retry and Fallback

The dispatcher implements a three-tier retry strategy. This is intentionally *inside* the dispatcher, not at the gateway level — the gateway sees a single `call` that either succeeds or returns a final error after all retry options are exhausted.

```
Dispatcher::call(lease, req)
  → attempt with current key
  → on retryable error (5xx, timeout):
      ├── retry same key (up to N times, with exponential backoff)
      └── on exhaustion: return error to gateway
          → gateway reports error (circuit breaker increments)
          → caller may retry via FallbackScheduler (different pool/provider)
```

> **Why not retry with a different key inside dispatcher?** Because key selection is the pool's responsibility. The dispatcher only knows about the current lease. Cross-key retry belongs at the `FallbackScheduler` level (see Natural Next Steps), where the full pool topology is visible.

> **Cancellation propagation.** When the upstream caller cancels (timeout, user disconnect), `tokio::select!` drops the dispatcher future, which drops the in-flight `reqwest` response future. reqwest's `Client` uses hyper under the hood — dropping the response future sends a `RST` on the TCP connection, so the provider stops processing. Without explicit cancellation, a dropped future may leave the TCP connection alive in the pool, causing phantom inflight: the provider is still working, consuming your TPM quota, but your `tpm_inflight` counter has already been decremented by the lease `Drop`.

---

## Decision Log

| Decision | Choice | Rejected alternative |
|---|---|---|
| TPM quota return | RAII Drop on KeyLease | Explicit release call — skippable on early return or async cancel |
| Key selection | Random-start first-fit | min_by_key (thundering herd: N requests all pick the same "least-loaded" key) |
| CAS failure handling | Skip to next key, cap at MAX_CAS_ATTEMPTS | Spin-retry same key — CPU spin; unbounded scan — tail latency |
| RPM check ordering | After TPM reserve (lock-free path) | Inside CAS loop — requires Mutex inside CAS, worse tradeoff |
| Cost representation | u64 micro-dollar | f32/f64 — no atomic operations; float accumulation loses precision |
| Rate limit algorithm | Sliding window | governor (GCRA) — rejects legal bursts that OpenAI actually permits |
| Sliding window mutex | parking_lot::Mutex (per-key) | std::sync::Mutex — slower under contention, poisons on panic |
| Key injection | Header per request | provider.with_api_key() — rebuilds HTTP client, loses connection pool |
| Budget phase | Pre-reserve + settle | Post-record only — no protection against concurrent overspend |
| Key cooldown | AtomicU64 timestamp, lazy check in acquire | spawn timer — zombie timer risk, holds Arc beyond key lifetime |
| Key state transitions | AtomicU8 (HEALTHY / DEAD only) | Three-state with STATE_COOLING — unnecessary intermediate state when using timestamp |
| Cooldown decoupling | Separate cool_down_until + failure_cool_down_until | Single timestamp — 429 recovery + stale failure count triggers false breaker trip |
| Error resilience | Circuit breaker (consecutive_failures → failure_cool_down_until) | Ignore non-rate-limit errors — bad key absorbs traffic indefinitely |
| Pool topology | PoolRegistry: Provider → Model → KeyPool | Flat Vec\<Key\> — different models pollute each other's quotas |
| Retry strategy | Dispatcher retries same key; FallbackScheduler retries cross-pool | No retry — single attempt, caller handles everything |
| Cancellation | tokio::select! + CancellationToken | Drop-only — phantom inflight, provider continues processing |

---

## PoolRegistry — Provider → Model → KeyPool

Keys are not a flat list. Different models under the same provider have independent rate limits — a GPT-4o key's TPM quota is separate from its GPT-4o-mini quota. The `PoolRegistry` enforces this hierarchy:

```rust
// key/registry.rs

pub struct PoolRegistry {
    pools: HashMap<(ProviderId, ModelId), KeyPool>,
}

impl PoolRegistry {
    pub fn acquire(
        &self,
        provider: &ProviderId,
        model: &ModelId,
        estimated_tokens: u32,
    ) -> Option<KeyLease> {
        self.pools
            .get(&(provider.clone(), model.clone()))?
            .acquire(estimated_tokens)
    }
}
```

> **Why not just tag keys with a model?** Because the same API key string may appear in multiple pools with different limits. A single OpenAI key has separate RPM/TPM limits for GPT-4o vs. GPT-4o-mini. Flattening them into one pool would cause cross-model quota pollution — a burst of cheap mini requests could starve the GPT-4o quota, or vice versa.

`Gateway::call` changes accordingly:

```rust
let lease = self.registry.acquire(&req.provider, &req.model, est_tokens)
    .ok_or(Error::NoAvailableKey)?;
```

---

## Observability

A scheduling system without observability is a black box. Every `KeyInner` exposes the following metrics, readable without acquiring any lock:

```rust
// Per-key metrics (all atomic, zero-cost read)
struct KeyMetrics {
    tpm_inflight:       u32,     // current TPM occupation
    tpm_limit:          u32,     // configured cap
    rpm_remaining:      u32,     // SlidingWindow::remaining()
    cool_down_until:    u64,     // 0 = healthy
    failure_cool_down:  u64,     // 0 = healthy
    consecutive_fails:  u32,     // circuit breaker counter
    state:              u8,      // HEALTHY / DEAD
}
```

The gateway should expose a `/health` or equivalent endpoint that returns:

| Metric | Source | Why |
|---|---|---|
| Per-key inflight / limit | `tpm_inflight`, `tpm_limit` | Detect capacity saturation before it causes `NoAvailableKey` |
| Per-key RPM remaining | `SlidingWindow::remaining()` | Predict imminent RPM exhaustion |
| Per-key cooldown state | `cool_down_until`, `failure_cool_down_until` | Distinguish rate-limit cooling from error-driven cooling |
| Per-key failure count | `consecutive_failures` | Detect degrading backends before the breaker trips |
| Budget used / remaining | `BudgetTracker` | Cost alerting |
| Acquire failure rate | Gateway-level counter | The single most important system health signal |

> **Without these metrics you cannot distinguish** "system is healthy but idle" from "all keys are cooling down and every request fails instantly." Both look the same from the outside.

---

## Known Tradeoffs

**RPM transient saturation.** RPM is checked after TPM reservation. Under burst load, requests that pass TPM but fail RPM briefly inflate `tpm_inflight`, potentially causing other requests to see false "full" states. The window is microsecond-scale and self-healing. We accept this to keep the acquire path lock-free. See `Gateway::call` for detailed analysis.

**Token estimation and P99 latency.** `estimated_tokens` is inherently inaccurate — streaming responses, function calls, and reasoning tokens can be 2-10× the estimate. This doesn't break correctness (settle corrects the delta, provider enforces real limits), but it **degrades scheduling precision**: the TPM inflight counter understates real load, causing the pool to over-admit requests. The provider responds with elevated latency (soft throttling) rather than a clean 429. This is the primary driver of P99 latency degradation under load. Mitigation: reserve `estimated * OVERBOOK_FACTOR` (e.g. 1.3×) for the inflight counter; settle corrects the delta, so the only cost is slightly reduced theoretical throughput.

**Phantom inflight on cancellation.** When a future is dropped without explicit cancellation, the underlying TCP connection may remain alive in reqwest's pool. The provider continues processing the request, consuming real TPM quota, but the gateway has already decremented `tpm_inflight` via lease `Drop`. This causes under-counting. Mitigation: `CancellationToken` + `tokio::select!` in `Gateway::call`.

---

## Natural Next Steps

**Multi-provider fallback.** Add a `FallbackScheduler` that wraps the `PoolRegistry` and implements cross-provider retry: when the primary provider returns `NoAvailableKey` or is circuit-broken, transparently retry on a fallback provider with model mapping (e.g. `gpt-4o` → `claude-3.5-sonnet`).

**Per-tenant budget isolation.** Replace the single `BudgetTracker` with a `HashMap<TenantId, BudgetTracker>`. The gateway takes a tenant ID on each call and routes to the appropriate tracker.

**EWMA latency-aware scoring.** Track a per-key exponentially weighted moving average of response time. Use it as a secondary signal in the scan: prefer keys with lower latency when multiple keys have available capacity. This naturally routes away from degraded backends before the circuit breaker trips — critical for detecting provider soft-throttling that doesn't produce 429s.

**Cost-based model downgrade.** Add a `ModelRouter` that, when budget is <20% remaining, substitutes a cheaper model (e.g. `gpt-4o` → `gpt-4o-mini`) before calling `acquire`.

**Adaptive feedback loop.** The current system uses static strategies. A production control plane would dynamically adjust `OVERBOOK_FACTOR` based on observed estimation error, tune `CIRCUIT_BREAKER_THRESHOLD` based on per-key error rates, and auto-scale the key pool based on sustained saturation signals.
