---
id: task-prompt-cache-004
title: Emit Claude cache_control breakpoints
status: done
priority: P0
tags: [protocol, claude, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on:
  - task-prompt-cache-001
blocks:
  - task-prompt-cache-005
  - task-prompt-cache-006
  - task-prompt-cache-008
---

# Background
Claude prompt caching uses explicit `cache_control` at prefix boundaries across tools, system, and messages. Current Claude request emission has no cache_control support.

# Goal
- Emit Claude-native `cache_control` from typed prompt cache policy.
- Support explicit safe breakpoint placements defined by `contract.prompt_cache.policy`.
- Reject unsupported Required breakpoints before transport.

# Execution Steps
- [x] Add Claude policy-to-wire mapping for tools, system, messages, and content blocks where supported.
- [x] Map `PromptCacheRetention` to Claude TTL wire fields conservatively.
- [x] Ensure dynamic suffix content is not accidentally marked cacheable by default.
- [x] Add tests for Auto, EndOfTools, EndOfInstructions, EndOfMessage, and EndOfContentBlock where supported.
- [x] Add tests that Required unsupported placements return `ProtocolError::UnsupportedFeature`.
- [x] Add tests that BestEffort unsupported placements can be surfaced for loss handling by task 005.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Placement acceptance: emitted Claude JSON puts `cache_control` on the expected stable prefix boundary.
- TTL acceptance: retention values are mapped only when supported and otherwise handled per policy.
- Safety acceptance: dynamic user/RAG suffix content is not marked with cache_control by default.
- Error acceptance: unsupported Required behavior fails before transport.

## Task Completion Acceptance Criteria
- Claude request emission can express provider-native prompt caching without raw user JSON edits.
- No OpenAI-specific behavior is introduced in this task.
- Prefix Builder requirements discovered here are recorded for task 006.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: task 006 may need to constrain builder output if Claude placement rules are stricter than expected.
- Recommended action: prefer fewer supported breakpoints over unsafe broad cache_control placement.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: Claude Messages emits `cache_control` for supported tool/system/message/content-block breakpoints with conservative TTL mapping and Required placement validation.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `src/protocol.rs`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `contract.generation.transcoding`.
- Primary files: `src/protocol.rs`.
