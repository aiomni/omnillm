---
id: task-primitive-protocol-009
title: Add primitive streaming and realtime scaffold
status: done
priority: P1
tags: [primitive-protocol, streaming, realtime]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-004
  - task-primitive-protocol-005
  - task-primitive-protocol-006
  - task-primitive-protocol-007
  - task-primitive-protocol-008
blocks:
  - task-primitive-protocol-010
---

# Background
Primitive streaming and realtime must preserve provider-native event/frame order and content while using side-channel usage telemetry for budget settlement.

# Goal
- Add primitive SSE streaming for OpenAI Chat/Responses, Anthropic Messages, and Gemini streamGenerateContent.
- Add realtime scaffolds for OpenAI Realtime and Gemini Live with explicit capability gates.
- Preserve provider-native frames and settle budget on completed event, EOF, error, cancellation, or session close.

# Execution Steps
- [x] Define primitive stream return type and event preservation rules.
- [x] Implement SSE frame forwarding for provider-native streams.
- [x] Add usage side-channel events without replacing provider frames.
- [x] Add EOF, provider error, cancellation, and partial usage settlement tests.
- [x] Add realtime session scaffold with clear unsupported or feature-gated transport behavior if full WebSocket/WebRTC support is deferred.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Preservation acceptance: provider-native stream frames are returned in order.
- Budget acceptance: stream and realtime paths settle exactly once.
- Cancellation acceptance: cancellation refunds or partially settles according to observed usage.
- Realtime acceptance: unsupported realtime transport fails explicitly before ambiguous partial execution.

## Task Completion Acceptance Criteria
- SSE primitive streams work for at least one OpenAI, one Anthropic, and one Gemini fixture or have documented provider-specific blockers.
- Realtime support status is explicit for OpenAI Realtime and Gemini Live.
- Existing canonical stream behavior remains unchanged.

# Dynamic Adjustments
- Current discovery: full WebSocket/WebRTC support may require new dependencies or feature flags.
- Downstream impact: docs and examples must accurately state which streaming/realtime transports are fully implemented.
- Recommended action: do not claim full realtime support until integration tests prove session lifecycle and settlement behavior.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.
- Follow-up hardening: SSE cancellation without provider usage now refunds budget and is covered by `primitive_sse_stream_cancellation_refunds_without_usage`.
- Provider stream fixtures: OpenAI, Anthropic, and Gemini SSE usage extraction are covered by primitive protocol tests.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `capability.primitive_provider.execute`, `contract.provider.primitive_protocol`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- References: OpenAI Realtime, Gemini Live, Anthropic streaming, LangChain streaming model patterns.
- Primary files: `src/gateway.rs`, `src/dispatcher.rs`, `src/primitive.rs`, stream tests.
