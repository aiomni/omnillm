---
id: plan.prompt_cache.implementation
kind: plan
status: completed
source_specs:
  - contract.prompt_cache.policy
  - contract.generation.model
  - contract.generation.transcoding
  - contract.multi_endpoint.transcoding
  - capability.budget.cost
version: 1
---

# Plan: Prompt Cache 一等支持

## Implementation Status
- Status: completed on 2026-05-02.
- Completed tasks: `task-prompt-cache-001` through `task-prompt-cache-008`.
- Validation: `cargo fmt` and `cargo test` pass.
- Current Spec synchronized: `contract.prompt_cache.policy` implementation state is now `implemented`.

## Goal
在 OmniLLM 中提供 provider-neutral 的 Prompt Cache 一等支持，同时保留 OpenAI 与 Claude 的 provider-native 语义、可观测 usage telemetry 和安全降级行为。

## Non-Goals
- 不实现本地 prompt 内容缓存或响应缓存。
- 不用统一 bool 掩盖 OpenAI 自动 prefix cache 与 Claude 显式 `cache_control` 的差异。
- 不在缺少 provider-specific cache rate table 前假设缓存折扣。
- 不改变现有 Gateway 调用顺序和 key pool 语义。

## Source Specs And Requirements
- `contract.prompt_cache.policy`: 定义 PromptCachePolicy、provider mapping、telemetry、pricing 与降级约束。
- `contract.generation.model`: 当前 canonical model 中 `CacheSettings` 是 legacy hint，需要迁移到 typed policy。
- `contract.generation.transcoding`: provider wire mapping 必须按 BestEffort/Required 语义处理。
- `contract.multi_endpoint.transcoding`: ConversionReport loss semantics 需要承载不支持 cache 的降级信息。
- `capability.budget.cost`: cache 折扣必须由 provider usage telemetry 和 provider-specific rates 驱动。

## Assumptions
- OpenAI prompt caching 是服务端自动 prefix cache，可通过 request fields 影响路由或保留策略。
- Claude prompt caching 需要在 tools/system/messages prefix 边界发出 `cache_control`。
- 现有 `CapabilitySet.cache` 可以作为兼容入口，但不应作为最终一等 API。
- 初始实现优先保证 typed request emission 与 usage parse，再接入精细 pricing。

## Open Questions
- `PromptCacheRetention::Long` 对不同 provider 的精确 wire 值是否需要 feature gate。
- `PromptCacheKey::StablePrefixHash` 是否由库自动生成，还是只提供 helper 给调用方显式传入。
- Claude `Auto` breakpoint 默认落点应选 system 末尾、last stable message，还是由 Prefix Builder 决定。
- Cache pricing 是否应继续硬编码在 `pricing.rs`，还是抽出 provider pricing registry。

## Strategy Summary
采用兼容优先、telemetry 先行的分阶段路线：先扩展 canonical policy 和 usage，不改变现有调用；再分别实现 OpenAI 与 Claude native mapping；随后补齐 Prefix Builder 和 pricing；最后清理 legacy `CacheSettings` 的语义边界。

## Technical Approach
- 在 `types.rs` 增加 `PromptCachePolicy`、`PromptCacheKey`、`PromptCacheRetention`、`CacheBreakpoint`、`PromptCacheUsage`，并把 `CapabilitySet` / `TokenUsage` 接到新类型。
- 在 protocol emit 层实现 OpenAI top-level fields 与 Claude `cache_control` breakpoint 映射。
- 在 response parse 层解析 OpenAI cached tokens 与 Claude cache read/write tokens。
- 在 `api_protocol.rs` 的 generation bridge sanitize 路径接入 BestEffort loss report 与 Required error。
- 在 Prefix Builder 中固定 stable prefix 与 dynamic suffix 的布局，避免调用方误破坏命中率。
- 在 pricing 层只在 provider usage 与 rate table 都可用后计算 cache-aware actual cost。

## Tradeoffs
| Decision | Chosen Route | Rejected Alternative | Rationale | Reversal Signal |
| --- | --- | --- | --- | --- |
| API 抽象 | `PromptCachePolicy` + provider mapping | `cache.enabled: bool` | bool 无法表达 Required、breakpoint、retention、telemetry | 多数 provider 都收敛到同一简单开关 |
| OpenAI 支持 | 映射 key/retention，不做 breakpoint | 强行模拟 breakpoint | OpenAI 依赖自动 prefix matching | OpenAI 增加显式 breakpoint API |
| Claude 支持 | 显式 `cache_control` breakpoint | 只做 top-level 自动缓存 | Claude 的精确收益来自 prefix boundary | Claude 弃用 content block cache_control |
| Pricing 顺序 | telemetry 先行，pricing 后接 | 立即硬编码折扣 | 避免价格变化导致错误计费 | 项目引入可更新 provider pricing registry |
| 降级语义 | BestEffort lossy，Required error | 全部静默忽略 | 防止成本敏感场景误以为启用缓存 | 用户明确要求完全静默兼容 |

## Phases
### Phase 1: Canonical Types And Compatibility
- Output: 新 prompt cache types 可序列化，旧 `CacheSettings` 可迁移到 BestEffort/Disabled。
- Depends on: `contract.prompt_cache.policy`。
- Unlocks: Provider-specific emission 和 usage parse。
- Validation: serde roundtrip、`CapabilitySet::is_empty`、legacy migration 单测通过。

### Phase 2: Usage Telemetry Parse
- Output: OpenAI cached tokens 与 Claude cache read/write tokens 进入 canonical `TokenUsage`。
- Depends on: Phase 1。
- Unlocks: cache-aware observability 与后续 pricing。
- Validation: OpenAI Responses/Chat、Claude Messages response fixture 单测覆盖 usage parse。

### Phase 3: Provider-Native Request Emission
- Output: OpenAI `prompt_cache_key` / `prompt_cache_retention` 和 Claude `cache_control` 可按 policy 发出。
- Depends on: Phase 1。
- Unlocks: Gateway 与 raw transcode 的 typed prompt cache 支持。
- Validation: wire JSON snapshot 覆盖 Disabled、BestEffort、Required、unsupported breakpoint。

### Phase 4: Bridge Semantics And Prefix Builder
- Output: `api_protocol.rs` 对 unsupported cache policy 生成 loss report 或 error，Prefix Builder 固化 stable prefix 布局。
- Depends on: Phase 2, Phase 3。
- Unlocks: 用户可安全构造跨 provider cacheable request。
- Validation: 转码矩阵覆盖 OpenAI、Claude、Gemini unsupported；builder 输出顺序稳定。

### Phase 5: Cache-Aware Pricing And Docs
- Output: `pricing.rs` 可在 usage telemetry 和 provider rate table 存在时计算 cache-aware actual cost，文档说明限制与迁移路径。
- Depends on: Phase 2。
- Unlocks: 成本统计闭环。
- Validation: cache read/write/cached token cost 单测、未知 rate fallback 单测、README/website/skill 同步检查。

## Dependencies
### Hard Dependencies
- `contract.prompt_cache.policy` 必须保持 Required 与 BestEffort 的差异。
- `TokenUsage` 必须能保留 provider cache telemetry，不能只暴露总 token。
- Protocol emit 必须能区分 OpenAI 自动 prefix cache 与 Claude 显式 `cache_control`。

### Soft Dependencies
- Provider fixture 覆盖 OpenAI cached tokens 与 Claude cache usage。
- Prefix hashing helper 可复用现有 JSON serialization 保证稳定性。
- Provider support matrix 可增加 prompt cache capability metadata。

### External Dependencies
- OpenAI 和 Anthropic prompt caching wire fields 与计费口径可能变化，需要 release 前复查官方文档。
- Provider pricing 可能随模型变化，需要避免把 cache discount 作为不可变常量。

## Risks And Mitigations
| Risk | Impact | Mitigation | Validation Or Rollback |
| --- | --- | --- | --- |
| Cache key 跨租户复用 | 数据隔离与成本归因风险 | key material 强制包含 tenant scope 或由调用方显式传入 | 单测检查 generated key 不含动态内容且含 namespace |
| 用户误以为 BestEffort 必定命中 | 成本和延迟预期错误 | 文档和 ConversionReport 明确只表示尝试启用 | telemetry 中 cached/read tokens 为唯一命中信号 |
| Claude breakpoint 放错位置 | 缓存无效或缓存动态内容 | Prefix Builder 默认只缓存 stable prefix | fixture 覆盖动态 suffix 不带 cache_control |
| Pricing 过早折扣 | budget 低估实际成本 | 估算阶段不应用 cache discount | pricing 单测验证 estimate 使用 uncached rate |
| Provider field drift | wire 请求被拒或 telemetry 解析失败 | provider-specific mapping 集中在 protocol 层 | live/fixture tests 失败时禁用对应 mapping |

## Validation Strategy
- Unit tests: canonical types serde、policy migration、cache key generation、usage parse、loss semantics。
- Protocol tests: OpenAI Responses/Chat emit fields，Claude Messages cache_control placements，Gemini unsupported behavior。
- Gateway tests: cached usage settle path不改变 key lease、RPM、budget refund 不变量。
- Matrix tests: `transcode_api_request` 对 BestEffort 与 Required 的 bridged/lossy/error 行为。
- Documentation checks: README、website docs、skill reference 同步 prompt cache 支持状态。

## Rollback Strategy
- 可先保留新类型但关闭 provider emission，所有 policy 作为 BestEffort loss report。
- 如果 pricing 出现偏差，回退到 uncached pricing，同时保留 telemetry 字段。
- 如果 Claude breakpoint mapping 不稳定，先降级为 OpenAI-only typed support，并让 Claude Required 返回 UnsupportedFeature。

## Spec Patch Needs
- None. 当前 Plan 已基于 `contract.prompt_cache.policy` 和同步后的相关 current Specs。

## Task Candidates
- 定义 prompt cache canonical types: 输出 `types.rs` 类型、serde、migration 单测。
- 实现 usage telemetry parse: 输出 OpenAI/Claude cached token parse 与 fixtures。
- 实现 OpenAI prompt cache emission: 输出 key/retention wire mapping 与 tests。
- 实现 Claude cache_control emission: 输出 breakpoint mapping 与 tests。
- 接入 bridge loss semantics: 输出 BestEffort/Required 转码行为。
- 增加 Prefix Builder: 输出 stable prefix layout helper 与安全 key generation。
- 接入 cache-aware pricing: 输出 provider-specific cache rate 处理与 fallback。
- 同步 docs 和 skill reference: 输出 README、website、skill 更新。
