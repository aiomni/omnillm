---
id: task-prompt-cache-002
title: Parse prompt cache usage telemetry
status: done
priority: P0
tags: [protocol, telemetry, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on:
  - task-prompt-cache-001
blocks:
  - task-prompt-cache-007
  - task-prompt-cache-008
---

# Background
Provider cache hits and cache writes must be observed from provider usage payloads. Current `TokenUsage` has only prompt, completion, and total token fields, so OpenAI cached tokens and Claude cache read/write tokens are lost.

# Goal
- Parse OpenAI prompt cache usage telemetry into canonical `TokenUsage`.
- Parse Claude prompt cache read/write usage telemetry into canonical `TokenUsage`.
- Preserve existing token totals and response parsing behavior.

# Execution Steps
- [x] Add OpenAI Responses usage parsing for `usage.prompt_tokens_details.cached_tokens`.
- [x] Add OpenAI Chat Completions usage parsing for `usage.prompt_tokens_details.cached_tokens` when present.
- [x] Add Claude Messages usage parsing for cache read/write token fields.
- [x] Add emission support for cache telemetry fields only where response emitters are expected to roundtrip canonical usage.
- [x] Add fixtures or unit tests for OpenAI Responses, OpenAI Chat Completions, and Claude Messages cache usage payloads.
- [x] Confirm streaming `Usage` event parsing preserves cache telemetry when providers include it.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- OpenAI acceptance: cached input tokens are present in canonical usage when OpenAI response JSON includes cached token details.
- Claude acceptance: cache read and creation tokens are present in canonical usage when Claude response JSON includes them.
- Compatibility acceptance: responses without cache usage still parse exactly as before.
- Streaming acceptance: provider stream usage events either preserve cache telemetry or explicitly document unsupported stream telemetry in this task.

## Task Completion Acceptance Criteria
- Targeted protocol tests pass for parse and emit paths touched by this task.
- No request emission behavior is added in this task.
- Pricing remains unchanged except for preserving new usage fields.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: task 007 depends on exact `PromptCacheUsage` field names and semantics.
- Recommended action: update task 007 if provider telemetry shape differs from the Spec.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: OpenAI cached-token telemetry and Claude cache read/write telemetry parse into `TokenUsage.prompt_cache`; response/stream usage emission preserves supported fields.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `src/protocol.rs`, `src/types.rs`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `contract.generation.transcoding`.
- Primary files: `src/protocol.rs`, `src/types.rs`.
