# Prompt Cache 一等支持

## Scope
- Add provider-neutral prompt cache policy support for OpenAI and Claude.
- Preserve provider-native semantics for OpenAI automatic prefix cache and Claude explicit `cache_control`.
- Parse provider cache usage telemetry and feed cache-aware cost accounting.
- Document support, limitations, and migration from legacy `CacheSettings`.

## Out Of Scope
- Local response caching.
- Local prompt KV caching.
- Silent cache enablement without policy or loss/error reporting.
- Pricing discounts before provider usage telemetry confirms cache behavior.

## Constraints
- Source of truth: `contract.prompt_cache.policy`.
- Required policy must fail before transport when the target provider cannot express requested semantics.
- BestEffort policy may degrade only with explicit `ConversionReport.loss_reasons`.
- Cache keys must not include raw API keys or sensitive dynamic user/RAG content.
- Estimate-time pricing must not assume cache hits.

## Current State
- Done: `LlmRequest.capabilities.prompt_cache` supports typed `PromptCachePolicy` with legacy `CapabilitySet.cache` migration.
- Done: OpenAI emits typed `prompt_cache_key` / `prompt_cache_retention`; Claude emits `cache_control` for supported breakpoints.
- Done: OpenAI and Claude usage telemetry is preserved in `TokenUsage.prompt_cache`.
- Done: `api_protocol.rs` reports BestEffort loss and rejects unsupported Required prompt cache policy before transport.
- Done: `PromptLayoutBuilder` helps construct stable-prefix-first requests and deterministic tenant-scoped keys.
- Done: cache-aware actual pricing is applied only with provider telemetry and known rates; estimates remain uncached/conservative.
- Done: README, website docs, and skill reference describe support and limitations.

## Milestones
- Canonical model and telemetry: done (`task-prompt-cache-001`, `task-prompt-cache-002`)
- Provider-native emission: done (`task-prompt-cache-003`, `task-prompt-cache-004`)
- Safe cross-provider behavior: done (`task-prompt-cache-005`, `task-prompt-cache-006`)
- Cost and release readiness: done (`task-prompt-cache-007`, `task-prompt-cache-008`)

## Key Dependencies
- All dependencies are resolved.
- Documentation reflects implemented behavior, not aspirational provider support.
- Task dashboard and individual task cards are synchronized.

## Acceptance Signals
- Canonical policy roundtrips through serde and preserves legacy compatibility.
- OpenAI and Claude tests prove request emission and usage parsing.
- Cross-provider transcode tests show BestEffort loss and Required errors.
- Pricing tests prove no pre-response cache discount and correct actual-cost behavior when cache telemetry exists.
- `cargo fmt` and `cargo test` pass.

## Final Acceptance Criteria
- The implementation satisfies `contract.prompt_cache.policy` without changing unrelated Gateway/key-pool behavior.
- All prompt-cache task cards are reviewed and synchronized with this project page and `index.md`.
- Documentation and skill reference state exactly what is typed support, what remains best-effort, and how to observe cache hits.
- Status: completed on 2026-05-02.
