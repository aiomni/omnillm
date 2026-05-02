---
id: task-primitive-provider-scope-008
title: Implement binary chunk streaming transport
status: done
priority: P1
tags: [primitive-provider-scope, streaming, binary]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-001
  - task-primitive-provider-scope-005
blocks:
  - task-primitive-provider-scope-009
  - task-primitive-provider-scope-010
---

# Background
Binary chunk streaming is needed for provider-native media streams such as OpenAI Audio Speech, where bytes should be consumed incrementally instead of after the full response body is buffered.

# Goal
- Implement `PrimitiveStreamMode::BinaryChunks` as a real transport path.
- Emit provider-native binary chunks without converting them to canonical text or SSE frames.
- Settle budget once on EOF, provider error, or cancellation.

# Execution Steps
- [x] Add dispatcher support for binary response byte streams.
- [x] Emit `PrimitiveStreamEvent::BinaryChunk` with media type metadata where available.
- [x] Add cancellation, EOF, provider error, and partial usage/billable-unit fallback tests.
- [x] Add OpenAI Audio Speech binary chunk fixture.
- [x] Update docs to distinguish full binary response from binary chunk streaming.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Transport acceptance: binary stream does not go through SSE parser or UTF-8 text conversion.
- Budget acceptance: cancellation refunds before usage and settles partial or reserved estimate after observed usage.
- Payload acceptance: chunk bytes are preserved exactly.

## Task Completion Acceptance Criteria
- OpenAI Audio Speech binary chunk streaming is test-backed.
- Existing non-stream `AudioSpeech` binary response behavior remains unchanged.
- Docs and support matrix distinguish binary response and binary chunk streaming.

# Dynamic Adjustments
- Current discovery: not all providers expose usage for media streams.
- Downstream impact: realtime task 009 can reuse settlement lessons from binary streaming.
- Recommended action: keep provider-specific media billing as billable-unit telemetry when available.

# Execution Log
## 2026-05-02
- Implemented `PrimitiveStreamMode::BinaryChunks` in gateway and dispatcher.
- Added OpenAI Audio Speech binary chunk fixture preserving bytes and media type while settling budget.
- Validation: `cargo fmt` and `cargo test primitive --tests` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; binary chunk streaming transport acceptance criteria are satisfied.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source spec tier: `support_tiers.p3_transport_expansion.OpenAi.AudioSpeechBinaryChunks`.
- Primary files: `src/dispatcher.rs`, `src/gateway.rs`, `src/primitive.rs`, `tests/primitive_protocol.rs`.
