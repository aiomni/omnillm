---
id: task-primitive-protocol-002
title: Define primitive public model and mode boundary
status: done
priority: P0
tags: [primitive-protocol, public-api, model]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-001
blocks:
  - task-primitive-protocol-003
  - task-primitive-protocol-004
  - task-primitive-protocol-005
  - task-primitive-protocol-006
  - task-primitive-protocol-007
  - task-primitive-protocol-008
  - task-primitive-protocol-009
  - task-primitive-protocol-010
---

# Background
Primitive protocol support needs a separate public model so provider-native payloads are not forced into canonical `LlmRequest`, `LlmResponse`, `ApiRequest`, or `ApiResponse` shapes.

# Goal
- Add public primitive types for provider, endpoint, wire format, request, response, stream event, stream mode, usage telemetry, and endpoint configuration.
- Define the explicit boundary between canonical `OpenAiResponsesStandard` mode and `ProviderPrimitive` mode.
- Re-export primitive public API without changing existing canonical API imports.

# Execution Steps
- [x] Define primitive provider/endpoint/wire-format enums aligned with `contract.provider.primitive_protocol`.
- [x] Define primitive request and response models around transport-native body, headers, query, method, path, stream mode, and metadata.
- [x] Define primitive stream/realtime event scaffolds without implementing full transport behavior in this task.
- [x] Define primitive usage telemetry as a side-channel model that can reference token usage without mutating payloads.
- [x] Re-export additive primitive types from the crate root.
- [x] Add serde/public API tests for the primitive model.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Shape acceptance: primitive request/response types can represent JSON, text, binary, multipart, SSE, WebSocket, WebRTC, and custom HTTP targets through existing or planned transport body types.
- Boundary acceptance: primitive types do not implement implicit conversion into canonical request or response types.
- Reexport acceptance: crate users can import primitive types from `omnillm` without breaking existing imports.
- Serialization acceptance: serde shapes are stable enough for fixtures and replay-style tests.

## Task Completion Acceptance Criteria
- Public API tests cover primitive type construction and serde roundtrip.
- Existing canonical API tests continue to pass.
- No provider-specific dispatch behavior is implemented in this task.

# Dynamic Adjustments
- Current discovery: confirm whether existing `RequestBody` and `ResponseBody` need small extensions before writing provider slices.
- Downstream impact: type names and field paths become dependencies for tasks 003 through 010.
- Recommended action: update `contract.public_api.surface` if public exports differ from the current Spec.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `contract.provider.primitive_protocol`, `system.omnillm.runtime`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Primary files: `src/primitive.rs`, `src/lib.rs`, `tests/api_surface.rs`.
