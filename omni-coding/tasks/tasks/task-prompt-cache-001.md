---
id: task-prompt-cache-001
title: Define prompt cache canonical types
status: done
priority: P0
tags: [api, types, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on: []
blocks:
  - task-prompt-cache-002
  - task-prompt-cache-003
  - task-prompt-cache-004
  - task-prompt-cache-005
  - task-prompt-cache-006
  - task-prompt-cache-007
  - task-prompt-cache-008
---

# Background
`contract.prompt_cache.policy` defines a typed `PromptCachePolicy`, but current code only exposes `CapabilitySet.cache: Option<CacheSettings>` as a legacy hint. Downstream provider emission, telemetry parsing, bridge semantics, pricing, and docs need a stable canonical shape first.

# Goal
- Add canonical prompt cache policy and telemetry types in `src/types.rs`.
- Preserve compatibility with existing `CacheSettings` while making it a legacy migration path.
- Keep `CapabilitySet::is_empty` and serde behavior correct.

# Execution Steps
- [x] Add `PromptCachePolicy`, `PromptCacheKey`, `PromptCacheRetention`, `CacheBreakpoint`, and `PromptCacheUsage` to `src/types.rs`.
- [x] Attach the policy to `CapabilitySet` using the path chosen by `contract.prompt_cache.policy`, while preserving or migrating `cache: Option<CacheSettings>`.
- [x] Attach prompt cache telemetry to `TokenUsage` without breaking `TokenUsage::total`.
- [x] Add conversion or helper behavior that maps legacy `CacheSettings.enabled` to Disabled or BestEffort.
- [x] Update public reexports in `src/lib.rs` for newly public types.
- [x] Add focused unit tests for serde roundtrip, legacy migration, and empty/default behavior.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Type-definition acceptance: all prompt cache types compile, derive the same public traits expected for nearby canonical types, and use serde shapes consistent with existing tagged enums.
- Compatibility acceptance: existing code using `CacheSettings` still compiles or has an intentional migration path recorded in this task.
- Usage acceptance: `TokenUsage::total` remains prompt plus completion when `total_tokens` is absent and is not affected by cache telemetry fields.
- Reexport acceptance: crate-root users can import the new public types from `omnillm`.

## Task Completion Acceptance Criteria
- `cargo test` or targeted equivalent passes for `types`-adjacent tests.
- No provider-specific wire behavior is implemented in this task.
- Downstream task dependencies and `index.md` are updated if the canonical API shape changes from the Plan.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: any change to type names or attachment paths must be propagated to tasks 002 through 008.
- Recommended action: keep this task focused on canonical shape and compatibility only.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: Canonical prompt cache policy, usage telemetry, legacy `CacheSettings` migration, public reexports, and `PromptLayoutBuilder` tests implemented.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `src/types.rs`, `src/lib.rs`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `contract.generation.model`.
- Primary files: `src/types.rs`, `src/lib.rs`.
