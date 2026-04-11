---
title: 架构说明
description: 在阅读源码之前，先了解 Key 池获取模型、租约生命周期和预算跟踪器设计。
label: 架构说明
release: v0.1.1
updated: 2026 年 4 月
summary: 从随机起点获取 Key、显式冷却语义，以及覆盖整个运行时的定点预算结算。
---

# 架构说明

> 面向 LLM API 访问的生产级 Rust 调度内核。
> 负责多 Key 负载均衡、单 Key 限流、成本跟踪以及错误驱动的状态管理，
> 并保证所有关键路径都具备并发安全性。
>
> 注意：当前对外请求/响应模型已经迁移到规范化的
> `Responses + Capability Layer` 混合层。下面这套并发、Key 池与预算架构仍然适用，
> 但部分请求/响应示例来自迁移前的 Chat Completions 模型。

---

## 设计原则

**Key 是租约，不是静态配置。** Key 会在使用前获取，在使用后通过 `Drop` 无条件归还。不存在“消费掉但没归还”的代码路径。

**计量是第一等公民。** 每个请求都会先占用配额（token 和预算），响应返回后再按实际使用量结算。错误会立即触发 Key 状态迁移。

**算法选择服从上游语义。** OpenAI 的 RPM 是滑动窗口，不是令牌桶，也不是 GCRA。限流器描述的是上游 provider 的真实行为，而不是为了实现方便做出的近似模型。

> ⚠️ **选择并预留必须是原子的。** 任何把 key 选择和配额预留拆开的系统都会留下一个 TOCTOU 窗口。这里的设计把两者合并进 `KeyPool::acquire` 内部的单个 CAS 循环中。
>
> ⚠️ **精度与无锁不是对立面。** 每个 key 独享的 `parking_lot::Mutex` 不存在跨 key 竞争，持锁时间是微秒级，在无竞争时几乎没有成本；基于时间戳的冷却既具备毫秒级精度，也完全无锁。这里并不存在“二选一”的取舍。

---

## 模块布局

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

### 请求流转

每次调用都严格遵循下面这条路径，没有例外。

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

## KeyLease：RAII 租约

这是这套设计中最核心的点。`KeyLease` 代表针对某个具体 `KeyInner` 所持有的一份 TPM 配额预留；当它被销毁时，无论调用成功、panic 还是被取消，配额都会通过 `fetch_sub` 归还，不存在“忘记释放”的路径。

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

> **为什么不用显式 `release()` 的 guard 守卫模式？** 因为显式释放是会被忘掉的。任何提前返回、`?` 传播或者 future 被取消时，都可能跳过那一步。只有 `Drop` 能在 Tokio 的异步取消语义下仍然提供保证。

---

## KeyPool — 获取与错误上报

这个 Key 池使用 **随机起点 first-fit** 策略：每个请求从一个随机下标开始扫描，绕一圈后选择第一个健康且容量足够的 key。这样可以避免 `min_by_key` 带来的羊群效应，即 N 个并发请求都看到同一个“当前最空闲”的 key，然后同时冲向同一个 CAS；同时也能以 O(1) 的均摊代价自然地把请求分散到不同 key 上。

扫描过程被 **`MAX_CAS_ATTEMPTS`**（默认 5）限制住。极端竞争且 key 数量很少时，无界扫描会退化成尾部非常差的自旋；加上上限后，这种情况会变成一次快速且有界的失败，调用方拿到 `None` 后可以在更高层重试（例如 `FallbackScheduler`），而不是继续烧 CPU。

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

> **为什么用时间戳而不是 `tokio::spawn`？** 基于额外异步任务的实现会带来僵尸计时器问题：被派生出的任务会持有一个超出请求生命周期的 `Arc<KeyInner>` 和定时器句柄。如果 Key 池被销毁，或者 key 被重新配置，这些计时器仍会继续在过期状态上运行。时间戳方案则是在 `acquire()` 时惰性检查，没有额外异步开销，也没有额外内存开销；一旦 `current_millis()` 超过截止时间，这个 key 就会自动重新可用。

> **解耦冷却。** 限流冷却（`cool_down_until`）和熔断冷却（`failure_cool_down_until`）是两份独立时间戳，并在 `acquire()` 中分别检查。这样可以避免一个非常隐蔽的问题：如果某个 key 累积了 4 次失败，随后又因为 429 冷却了 60 秒，恢复后的第一笔请求又遇到一次 5xx，那么共享计数器会立刻把它判成第 5 次失败并再次触发熔断，尽管这些错误其实分散在很长时间内。分离时间戳、分离计数器，也就分离了语义。

> ⚠️ **CAS 竞争。** 随机起点、失败即跳过，再加上 `MAX_CAS_ATTEMPTS` 上限后，CAS 竞争在概率意义和绝对次数上都被约束住了。每次获取最多尝试 5 次 CAS，然后就返回 `None` 交给更高层处理。如果你有上千个并发调用者而 key 又很少，应该考虑把 Key 池做分片。

---

## SlidingWindow：RPM 与 TPM 限流控制

每个 key 都会带两份 `SlidingWindow`，一份对应 RPM，一份对应 TPM。滑动窗口能够准确刻画 OpenAI 的真实限流行为，而不是像 GCRA 那样偏保守，或者像固定窗口那样允许边界突刺。

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

> **为什么不用 `governor`？** `governor` 库使用的是 GCRA，会强制请求之间保持均匀间隔。但 OpenAI 的 RPM 允许窗口内突发，也就是在一分钟窗口的第一秒里打进 60 个请求是合法的，而 GCRA 会把这种流量直接拒掉。这里的滑动窗口可以自然容纳这种突发。

> **Mutex 选型。** 这里的 `Mutex` 持锁时间是微秒级的（本质上就是一段 `pop_front` 循环），真正该关心的不是“要不要加锁”，而是锁的实现本身。`parking_lot::Mutex` 比 `std::sync::Mutex` 在竞争下快约 3 到 5 倍，不会因为 panic 进入 poisoned 状态，而且在异步上下文里也不会长时间阻塞 Tokio worker 线程。再加上每个 `SlidingWindow` 都由单个 key 独享，所以压根没有跨 key 的锁竞争。

> ⚠️ **内存上界。** `VecDeque<Instant>` 最多会持有 `limit` 条记录。对于 RPM=10000，大约就是每个 key 80KB。几十个 key 完全可以接受；如果你有成千上万个 key，就该考虑换成固定容量的环形缓冲区。

---

## BudgetTracker：定点数、无锁

成本以 `u64` 微美元（1 USD = 1,000,000 单位）的形式存储。这样既避免了浮点误差，也能直接使用原子 CAS 操作。两阶段结算用于修正“预估成本”和“实际使用”之间的差值。

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

## Gateway::call — 主执行路径

Gateway 把这些组件全部串了起来。操作顺序本身非常重要：必须先获取 key 再占预算（否则获取失败也会消耗预算），并且必须在 lease 释放之前完成结算（这样计量逻辑运行时 key 仍然处于已预留状态）。

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

> **Dispatcher 通过请求头注入 key，而不是克隆客户端实例。** `Dispatcher` 内部只持有一个 `reqwest::Client`，也就是一份共享连接池。API key 在每次请求上通过 `Authorization: Bearer {lease.inner.key}` 注入。上游 provider 自身保持无状态。

> ⚠️ **RPM 的权衡。** RPM 检查发生在 TPM 预留之后。在突发负载下（例如 1000 个并发请求），那些通过了 TPM 但没通过 RPM 的请求会在 lease 释放前短暂抬高 `tpm_inflight`。这种瞬时饱和会导致别的请求错误地看到“池已经满了”，于是直接返回 `None`。这个窗口虽然只有微秒级，而且会自动恢复，但在极端突发场景下会放大尾延迟。我们接受这个权衡，是因为要保持获取路径无锁；如果把 RPM 合并进 CAS 循环，就不得不在 CAS 内部持有 Mutex，而那会更糟。

### Dispatcher — 重试与回退

Dispatcher 实现了一套三层重试策略，而且这部分逻辑被刻意放在 `Dispatcher` 内部，而不是 `Gateway` 层。也就是说，从 `Gateway` 视角看出去，只有一次 `call`，要么成功，要么在所有重试选项耗尽后返回最终错误。

```
Dispatcher::call(lease, req)
  → attempt with current key
  → on retryable error (5xx, timeout):
      ├── retry same key (up to N times, with exponential backoff)
      └── on exhaustion: return error to gateway
          → gateway reports error (circuit breaker increments)
          → caller may retry via FallbackScheduler (different pool/provider)
```

> **为什么不在 dispatcher 内部换一个 key 重试？** 因为 key 的选择本来就是 Key 池的职责。Dispatcher 只知道当前手上的这份 lease，并不知道整个池的拓扑。跨 key 重试应该属于 `FallbackScheduler` 这一层（见后面的“自然的下一步”），因为只有在那一层才能看见完整池结构。

> **取消传播。** 当上游调用方取消请求时（超时、用户断开连接），`tokio::select!` 会直接丢弃 `Dispatcher` 的 future，连带着也丢掉正在进行中的 `reqwest` 响应 future。reqwest 的 `Client` 底层基于 hyper，丢弃响应 future 会在 TCP 连接上发送一个 `RST`，从而让 provider 停止处理请求。没有显式取消的话，一个被丢弃的 future 可能会把 TCP 连接继续留在池里，造成 `phantom inflight`：provider 还在继续工作、继续消耗你的 TPM 配额，但本地的 `tpm_inflight` 计数已经因为 lease 的 `Drop` 被减掉了。

---

## 决策记录

| 决策点 | 采用方案 | 被拒绝的替代方案 |
|---|---|---|
| TPM 配额归还 | 在 KeyLease 上使用 RAII `Drop` | 显式 `release()` 调用，容易在提前返回或异步取消时被跳过 |
| Key 选择 | 随机起点 first-fit | `min_by_key`，会导致 N 个请求都盯上同一个“最空闲” key |
| CAS 失败处理 | 跳到下一个 key，并限制 `MAX_CAS_ATTEMPTS` | 对同一个 key 自旋重试，或无界扫描，都会拉高 CPU 和尾延迟 |
| RPM 检查顺序 | TPM 预留之后再检查，保持 acquire 路径无锁 | 放进 CAS 循环内部，需要在 CAS 里持有 Mutex，权衡更差 |
| 成本表示 | `u64` micro-dollar | `f32` / `f64`，不能做原子操作且会累积精度损失 |
| 限流算法 | 滑动窗口 | `governor` 的 GCRA，会拒绝 OpenAI 实际允许的突发流量 |
| Sliding window 的 Mutex | `parking_lot::Mutex`（每个 key 独享） | `std::sync::Mutex`，竞争下更慢，panic 后还会 poisoned |
| Key 注入方式 | 每个请求单独通过请求头注入 | `provider.with_api_key()`，会重建 HTTP client 并丢失连接池 |
| 预算阶段 | 预留 + settle | 只在结束后记录，无法防止并发超支 |
| Key 冷却实现 | `AtomicU64` 时间戳，并在 acquire 时惰性检查 | 异步定时器，存在僵尸计时器风险，还会让 `Arc` 活得更久 |
| Key 状态迁移 | 只有 `HEALTHY / DEAD` 两态，冷却用时间戳表达 | 引入 `STATE_COOLING` 三态，在时间戳模型下没有必要 |
| 冷却解耦 | `cool_down_until` 与 `failure_cool_down_until` 分离 | 单一时间戳，容易在 429 恢复后被陈旧失败计数错误触发熔断 |
| 错误韧性 | 使用熔断器（`consecutive_failures` → `failure_cool_down_until`） | 忽略非限流错误，会让坏 key 一直吞流量 |
| 池拓扑 | `PoolRegistry: Provider → Model → KeyPool` | 扁平 `Vec<Key>`，会让不同模型互相污染配额 |
| 重试策略 | Dispatcher 重试同一个 key，`FallbackScheduler` 做跨池重试 | 完全不重试，让调用方处理一切 |
| 取消策略 | `tokio::select!` + `CancellationToken` | 只靠 Drop，容易产生 phantom inflight，provider 仍继续执行 |

---

## PoolRegistry：Provider → Model → KeyPool 层级

Key 不是一张扁平列表。同一个 provider 下的不同模型拥有彼此独立的限流额度，例如 GPT-4o 的 TPM 配额和 GPT-4o-mini 的 TPM 配额就是分开的。`PoolRegistry` 负责强制落实这一层级关系：

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

> **为什么不只是给 key 打一个模型标签？** 因为同一个 API key 字符串可能会出现在多个不同限制的池里。单个 OpenAI key 在 GPT-4o 和 GPT-4o-mini 上拥有不同的 RPM/TPM 限额。如果把它们拍平进同一个池，就会产生跨模型配额污染，例如大量便宜的 mini 请求可能把 GPT-4o 的配额挤掉，反过来也一样。

`Gateway::call` 也会随之变成：

```rust
let lease = self.registry.acquire(&req.provider, &req.model, est_tokens)
    .ok_or(Error::NoAvailableKey)?;
```

---

## 可观测性

一个没有可观测性的调度系统，本质上就是黑盒。每个 `KeyInner` 都会暴露下面这些无需加锁即可读取的指标：

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

Gateway 最好暴露一个 `/health` 或等价端点，返回如下信息：

| 指标 | 来源 | 作用 |
|---|---|---|
| 每个 key 的 inflight / limit | `tpm_inflight`, `tpm_limit` | 在 `NoAvailableKey` 出现前提前发现容量饱和 |
| 每个 key 的 RPM 剩余额度 | `SlidingWindow::remaining()` | 预测是否即将耗尽 RPM |
| 每个 key 的冷却状态 | `cool_down_until`, `failure_cool_down_until` | 区分“限流冷却”与“错误驱动冷却” |
| 每个 key 的失败计数 | `consecutive_failures` | 在熔断触发前发现后端正在恶化 |
| 预算已用 / 剩余 | `BudgetTracker` | 成本告警 |
| 获取失败率 | Gateway 级计数器 | 整个系统健康度最重要的单一指标 |

> **如果没有这些指标，你根本无法区分**“系统健康但暂时空闲”和“所有 key 都在冷却中，导致每个请求都会立刻失败”。从外部看，这两种状态几乎一模一样。

---

## 已知权衡

**RPM 瞬时饱和。** RPM 在 TPM 预留之后检查。突发负载下，那些通过 TPM 但没通过 RPM 的请求会短暂抬高 `tpm_inflight`，从而让其他请求看到一个错误的“已满”状态。这个窗口只有微秒级，并且会自动恢复。我们接受它，是为了保持获取路径无锁。更详细的分析见 `Gateway::call` 一节。

**Token 估算与 P99 延迟。** `estimated_tokens` 天生就不精确，尤其是流式响应、函数调用和推理 token，可能是估值的 2 到 10 倍。这不会破坏正确性，因为结算会修正差值，上游 provider 也会施加真实限额；但它会 **降低调度精度**：TPM inflight 计数低估了真实负载，从而让 Key 池错误地接纳更多请求。provider 往往不会直接返回 429，而是通过更高的延迟来做软限流。这正是高负载下 P99 延迟恶化的主要来源。缓解方式是对 inflight 预留使用 `estimated * OVERBOOK_FACTOR`（例如 1.3 倍）；最终结算仍然会修正差值，代价只是牺牲一点理论吞吐。

**取消时的 `phantom inflight`。** 如果 future 在没有显式取消的情况下被 drop，底层 TCP 连接可能仍然存活在 reqwest 的连接池里。provider 还在继续处理请求，继续消耗真实 TPM，但 Gateway 已经因为 lease `Drop` 把 `tpm_inflight` 减掉了，这就造成了低估。缓解方式是在 `Gateway::call` 中使用 `CancellationToken` + `tokio::select!`。

---

## 自然的下一步

**多 provider 回退。** 增加一个包裹 `PoolRegistry` 的 `FallbackScheduler`，实现跨 provider 重试：当主 provider 返回 `NoAvailableKey` 或已经熔断时，自动带着模型映射（例如 `gpt-4o` → `claude-3.5-sonnet`）切换到备用 provider。

**按租户隔离预算。** 把单一 `BudgetTracker` 替换成 `HashMap<TenantId, BudgetTracker>`。Gateway 在每次调用时接收 tenant ID，并路由到对应的预算跟踪器。

**基于 EWMA 的延迟感知评分。** 为每个 key 记录一条响应时间的指数滑动平均线，把它作为扫描时的次级信号：当多个 key 都可用时，优先选延迟更低的那个。这样可以在熔断器触发前就自然绕开正在恶化的后端，尤其适合发现那些不会返回 429、只会悄悄变慢的软限流。

**基于成本的模型降级。** 增加一个 `ModelRouter`，当预算剩余低于 20% 时，在调用 `acquire` 前自动把更贵的模型替换成更便宜的模型（例如 `gpt-4o` → `gpt-4o-mini`）。

**自适应反馈回路。** 当前系统使用的是静态策略。更成熟的生产控制平面会基于实际估算误差动态调整 `OVERBOOK_FACTOR`，基于每个 key 的错误率调优 `CIRCUIT_BREAKER_THRESHOLD`，并依据持续的饱和信号自动扩缩容 Key 池。
