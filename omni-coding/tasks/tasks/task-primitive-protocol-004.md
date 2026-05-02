---
id: task-primitive-protocol-004
title: Implement primitive non-stream execution path
status: done
priority: P0
tags: [primitive-protocol, gateway, dispatcher]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-002
  - task-primitive-protocol-003
blocks:
  - task-primitive-protocol-005
  - task-primitive-protocol-006
  - task-primitive-protocol-007
  - task-primitive-protocol-008
  - task-primitive-protocol-009
  - task-primitive-protocol-010
---

# Background
Primitive requests must go directly to provider-native endpoints without canonical parsing or emission. The first executable slice should support non-streaming transport while preserving existing Gateway behavior.

# Goal
- Add an explicit primitive non-streaming Gateway entrypoint.
- Build provider-native transport from primitive request fields plus auth/default headers/base URL/query metadata.
- Return provider-native response body without canonical conversion.
- Classify provider/network/cancel errors through existing Gateway error semantics.

# Execution Steps
- [x] Add primitive endpoint configuration and URL derivation rules.
- [x] Add primitive non-streaming dispatcher path for JSON, text, binary, and multipart request bodies.
- [x] Preserve primitive response status, headers, content type, and body.
- [x] Ensure registry support is checked before key/RPM/budget/network operations.
- [x] Add tests for auth injection, default headers, query parameters, base URL/path handling, and raw payload preservation.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Conversion acceptance: primitive dispatch never calls canonical parse/emit/transcode functions.
- Transport acceptance: only transport metadata, auth, default headers, query, timeout, and base URL resolution may be added.
- Preservation acceptance: successful primitive response body equals provider raw body for JSON/text/binary paths.
- Error acceptance: HTTP 401/403, 429, provider error, network error, and cancellation map to Gateway errors consistently.

## Task Completion Acceptance Criteria
- Non-stream primitive call path works for at least one OpenAI-compatible JSON endpoint in tests.
- Unsupported primitive endpoint fails before acquiring key or reserving budget.
- Existing canonical Gateway tests pass unchanged.

# Dynamic Adjustments
- Current discovery: multipart support may require enabling or adapting reqwest multipart features.
- Downstream impact: provider family tasks depend on this entrypoint and error model.
- Recommended action: keep streaming and realtime out of this task except for rejecting unsupported stream modes clearly.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `capability.primitive_provider.execute`, `contract.provider.primitive_protocol`, `contract.error.model`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Primary files: `src/gateway.rs`, `src/dispatcher.rs`, `src/primitive.rs`, `src/error.rs`.
