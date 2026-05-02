---
id: task-prompt-cache-005
title: Apply prompt cache bridge semantics
status: done
priority: P0
tags: [api-protocol, transcoding, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on:
  - task-prompt-cache-001
  - task-prompt-cache-003
  - task-prompt-cache-004
blocks:
  - task-prompt-cache-008
---

# Background
Prompt cache policy must not silently disappear during transcoding. `contract.prompt_cache.policy` requires BestEffort degradation to be reflected in `ConversionReport.loss_reasons`, while Required degradation must return an error before transport.

# Goal
- Enforce prompt cache BestEffort and Required behavior in `api_protocol.rs` generation bridge paths.
- Preserve existing non-cache loss semantics.
- Make unsupported providers and unsupported provider-specific shapes explicit.

# Execution Steps
- [x] Extend generation request sanitization to inspect prompt cache policy.
- [x] Add loss reasons when BestEffort policy cannot be represented by the target wire format.
- [x] Return an explicit error for Required policy when the target wire format cannot represent requested semantics.
- [x] Cover OpenAI, Claude, Gemini, and non-generation unsupported cases in tests.
- [x] Ensure loss reason deduplication still works with prompt cache loss reasons.
- [x] Update any affected `ConversionReport` expectations in existing tests.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- BestEffort acceptance: unsupported prompt cache policy produces `bridged=true`, `lossy=true`, and a specific loss reason.
- Required acceptance: unsupported prompt cache policy returns an error and does not emit a partial request.
- Compatibility acceptance: existing loss reasons for tools, reasoning, metadata, and vendor extensions remain intact.
- Matrix acceptance: target wire formats include OpenAI Responses, OpenAI Chat, Anthropic Messages, and Gemini GenerateContent.

## Task Completion Acceptance Criteria
- Cross-provider prompt cache behavior is deterministic and test-covered.
- No provider-native wire field mapping is introduced here beyond consuming behavior from tasks 003 and 004.
- Task 008 can document exact downgrade/error semantics from tests.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: docs must include any unsupported provider or unsupported breakpoint combinations discovered here.
- Recommended action: add newly discovered ambiguous provider behavior to `omni-coding/tasks/inbox.md` if it is not immediately task-worthy.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: Bridge semantics enforce BestEffort loss reasons and Required pre-transport errors across OpenAI, Claude, and Gemini target wire formats.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `src/api_protocol.rs`, `src/protocol.rs`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `contract.multi_endpoint.transcoding`, `contract.generation.transcoding`.
- Primary files: `src/api_protocol.rs`, `src/protocol.rs` if shared helpers are needed.
