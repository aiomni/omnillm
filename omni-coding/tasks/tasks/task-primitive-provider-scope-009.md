---
id: task-primitive-provider-scope-009
title: Implement realtime session transports
status: todo
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
- [ ] Define or refine realtime session runtime API based on existing `PrimitiveRealtimeSession` scaffold.
- [ ] Implement OpenAI Realtime WebSocket session tests with provider-native messages.
- [ ] Implement Gemini Live WebSocket session tests with provider-native messages.
- [ ] Define WebRTC support level as Planned or implement feature-gated tests if in scope.
- [ ] Add close, provider error, cancellation, accumulated usage, and no-usage fallback settlement tests.

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
- Not started.

# Review
- Review status: pending.

# Notes
- Source spec tier: `support_tiers.p3_transport_expansion`.
- Primary files: `src/gateway.rs`, `src/dispatcher.rs`, `src/primitive.rs`, realtime tests.
