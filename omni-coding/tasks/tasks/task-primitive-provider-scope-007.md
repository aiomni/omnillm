---
id: task-primitive-provider-scope-007
title: Implement P2 batch lifecycle providers
status: todo
priority: P1
tags: [primitive-provider-scope, batches, async-jobs]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-006
blocks:
  - task-primitive-provider-scope-010
---

# Background
OpenAI, Anthropic, and Gemini all have batch-style async processing surfaces that need explicit lifecycle and budget behavior.

# Goal
- Implement provider-native batch lifecycle support using the async job boundary from task 006.
- Cover create/get/list/cancel/result retrieval or equivalent operations per provider.
- Preserve raw provider payloads and settle budget from provider usage when available.

# Execution Steps
- [ ] Add OpenAI Batches lifecycle registry/path/runtime tests.
- [ ] Add Anthropic Message Batches lifecycle registry/path/runtime tests.
- [ ] Add Gemini Batches and Operations polling registry/path/runtime tests.
- [ ] Add provider error, cancellation, repeated poll, and result usage settlement tests.
- [ ] Update examples and docs with async job caveats.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Lifecycle acceptance: create, get/list, cancel, and results paths are distinguishable.
- Payload acceptance: provider-native batch request/response bodies are preserved.
- Budget acceptance: polling metadata does not consume token budget; result usage settles once when observed.

## Task Completion Acceptance Criteria
- Batch lifecycle support is test-backed for OpenAI, Anthropic, and Gemini.
- Unsupported lifecycle operations fail before ambiguous dispatch.
- Docs do not equate batch lifecycle with simple `primitive_call` support.

# Dynamic Adjustments
- Current discovery: provider result retrieval may involve files or operations resources.
- Downstream impact: final docs task 010 depends on exact supported lifecycle operations.
- Recommended action: start with create/get/cancel and add result retrieval only when usage semantics are testable.

# Execution Log
- Not started.

# Review
- Review status: pending.

# Notes
- Source spec tier: `support_tiers.p2_async_job_lifecycle`.
- Primary files: `src/primitive.rs`, `src/gateway.rs`, `src/dispatcher.rs`, `tests/primitive_protocol.rs`.
