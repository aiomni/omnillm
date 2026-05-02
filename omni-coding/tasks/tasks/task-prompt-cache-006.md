---
id: task-prompt-cache-006
title: Add prompt prefix builder
status: done
priority: P1
tags: [api, ergonomics, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on:
  - task-prompt-cache-001
  - task-prompt-cache-004
blocks:
  - task-prompt-cache-008
---

# Background
Prompt caching works only when stable content forms a consistent prefix and dynamic content stays in the suffix. Without a builder, users can accidentally place dynamic user/RAG content inside cacheable segments or generate unsafe cache keys.

# Goal
- Provide a helper that constructs cache-friendly `LlmRequest` layouts.
- Generate tenant-safe stable prefix cache keys when requested.
- Keep provider-specific prompt cache details out of user prompt construction where possible.

# Execution Steps
- [x] Design a `PromptLayoutBuilder` or `CachedPromptPrefix` API aligned with `contract.prompt_cache.policy`.
- [x] Ensure builder ordering follows tools, instructions, stable examples, stable context, conversation history, dynamic user input, dynamic RAG context.
- [x] Add stable prefix hash generation that excludes raw API keys, dynamic user input, and dynamic RAG content.
- [x] Attach the chosen `PromptCachePolicy` to the resulting `LlmRequest`.
- [x] Add tests that builder output is stable for identical prefixes and changes when stable prefix content changes.
- [x] Add tests that dynamic suffix changes do not alter generated stable prefix key.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Layout acceptance: built requests order stable segments before dynamic suffix segments.
- Key acceptance: generated key includes namespace/provider-relevant scope and excludes secrets and dynamic content.
- Policy acceptance: builder attaches cache policy without requiring users to manually edit provider wire JSON.
- Stability acceptance: deterministic inputs produce deterministic prefix hashes.

## Task Completion Acceptance Criteria
- Users can construct cacheable requests safely without knowing Claude `cache_control` placement details or OpenAI routing fields.
- Builder does not replace direct low-level API usage for advanced callers.
- Documentation task has concrete examples to publish.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: task 008 should document builder usage and pitfalls.
- Recommended action: if API surface grows too broad, split non-essential helpers into a later P2 task.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: `PromptLayoutBuilder` constructs stable-prefix-first requests and generates deterministic tenant-scoped keys that exclude dynamic suffix content.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `src/types.rs`, `src/lib.rs`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `contract.generation.model`.
- Primary files: likely `src/types.rs` or a new public helper module, plus `src/lib.rs` reexports.
