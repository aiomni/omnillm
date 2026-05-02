---
id: task-prompt-cache-003
title: Emit OpenAI prompt cache fields
status: done
priority: P0
tags: [protocol, openai, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on:
  - task-prompt-cache-001
blocks:
  - task-prompt-cache-005
  - task-prompt-cache-008
---

# Background
OpenAI prompt caching is service-side automatic prefix caching. The typed API should map provider-neutral policy to OpenAI top-level request fields where supported, instead of relying on raw `vendor_extensions` as the only escape hatch.

# Goal
- Emit OpenAI `prompt_cache_key` and `prompt_cache_retention` from typed prompt cache policy.
- Preserve OpenAI's automatic prefix-cache semantics without pretending to support explicit breakpoints.
- Ensure unsupported Required breakpoint behavior fails before transport.

# Execution Steps
- [x] Add OpenAI policy-to-wire mapping in OpenAI Responses request emission.
- [x] Add OpenAI policy-to-wire mapping in OpenAI Chat Completions request emission when applicable.
- [x] Ensure `CacheBreakpoint` values that OpenAI cannot express are ignored only for BestEffort with loss metadata handled by task 005, or error for Required.
- [x] Define conflict behavior when both typed policy and `vendor_extensions` specify OpenAI prompt cache fields.
- [x] Add request emission tests for Disabled, BestEffort with key, BestEffort with retention, Required with supported fields, and Required with unsupported breakpoint.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Responses acceptance: OpenAI Responses JSON contains expected top-level prompt cache fields for supported typed policies.
- Chat acceptance: OpenAI Chat JSON contains expected top-level prompt cache fields when supported by the chosen mapping.
- Conflict acceptance: typed policy vs `vendor_extensions` precedence is deterministic and covered by tests.
- Unsupported acceptance: unsupported Required semantics do not silently emit a partial request.

## Task Completion Acceptance Criteria
- OpenAI request emission is typed and no longer requires users to manually set prompt cache fields through `vendor_extensions`.
- No Claude `cache_control` behavior is implemented in this task.
- Task 005 has enough provider behavior to add bridge loss/error semantics.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: if OpenAI field names or retention values require feature gating, update task 005 and task 008.
- Recommended action: keep OpenAI breakpoint behavior explicit and conservative.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: OpenAI Responses and Chat emit typed `prompt_cache_key` / `prompt_cache_retention` with deterministic typed-field precedence and Required breakpoint errors.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `src/protocol.rs`, `src/api_protocol.rs`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `contract.generation.transcoding`.
- Primary files: `src/protocol.rs`, `src/api_protocol.rs` if transport reports need loss metadata coordination.
