---
id: task-primitive-provider-scope-009
title: Implement realtime session transports
status: done
priority: P1
tags: [primitive-provider-scope, realtime, websocket]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-001
  - task-primitive-provider-scope-008
blocks:
  - task-primitive-provider-scope-010
---

# Background
OpenAI Realtime and Gemini Live require session lifecycle transports and should remain explicit scaffolds until WebSocket/WebRTC behavior is implemented and tested.

# Goal
- Implement realtime session opening, event send/receive, close, cancellation, and usage settlement for supported transports.
- Start with WebSocket before WebRTC unless a current Spec changes priority.
- Keep realtime support separate from ordinary HTTP primitive calls.

# Execution Steps
- [x] Define or refine realtime session runtime API based on existing `PrimitiveRealtimeSession` scaffold.
- [x] Implement OpenAI Realtime WebSocket session tests with provider-native messages.
- [x] Implement Gemini Live WebSocket session tests with provider-native messages.
- [x] Define WebRTC support level as Planned or implement feature-gated tests if in scope.
- [x] Add close, provider error, cancellation, accumulated usage, and no-usage fallback settlement tests.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Lifecycle acceptance: sessions have explicit open/send/receive/close or equivalent operations.
- Transport acceptance: WebSocket/WebRTC paths do not route through HTTP `primitive_call`.
- Budget acceptance: each session settles once on close, provider error, cancellation, or terminal event.

## Task Completion Acceptance Criteria
- OpenAI Realtime and Gemini Live support levels are test-backed and documented.
- Scaffold errors remain for any realtime mode not implemented.
- Existing SSE and binary stream tests remain green.

# Dynamic Adjustments
- Current discovery: WebRTC may require feature flags or new dependencies.
- Downstream impact: final support claim audit must state exact transport support.
- Recommended action: implement WebSocket first and keep WebRTC Planned unless tests are reliable.

# Execution Log
- Implemented `PrimitiveRealtimeSession` event, usage, and metadata fields for completed session capture.
- Added dispatcher WebSocket transport with auth/header/query injection, initial JSON/text/binary send, provider-native message preservation, usage side-channel events, close handling, and request-timeout guarded open/receive operations.
- Added gateway budget/key/RPM integration for realtime sessions with once-only settlement on success, provider open error, local protocol error, and cancellation.
- Added OpenAI Realtime and Gemini Live local WebSocket tests for native messages, usage extraction, budget settlement, no-usage fallback, provider handshake error refund, cancellation refund, and WebRTC planned protocol error.
- Validation: `cargo test primitive --tests` passed.

# Review
- Review status: done.
- Task can be marked done: WebSocket support is test-backed for OpenAI Realtime and Gemini Live, while WebRTC remains an explicit planned/protocol-error path.
- Adjustment recorded: realtime API returns a completed session transcript rather than a long-lived interactive handle in this slice; richer incremental send/receive handles can be a future promoted task.

# Notes
- Source spec tier: `support_tiers.p3_transport_expansion`.
- Primary files: `src/gateway.rs`, `src/dispatcher.rs`, `src/primitive.rs`, realtime tests.
