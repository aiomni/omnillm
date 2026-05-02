---
id: task-primitive-protocol-006
title: Implement OpenAI primitive API family
status: done
priority: P1
tags: [primitive-protocol, openai, provider-slice]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-003
  - task-primitive-protocol-004
  - task-primitive-protocol-005
blocks:
  - task-primitive-protocol-009
  - task-primitive-protocol-010
---

# Background
OpenAI primitive support should follow the openai-go API family target while keeping OpenAI Responses as the canonical standard protocol for existing Gateway users.

# Goal
- Support OpenAI primitive calls for Responses, Chat Completions, Images, Audio Transcriptions, and Audio Speech.
- Add Realtime scaffold with explicit support status and transport limitations.
- Preserve OpenAI-native request and response payloads while extracting usage telemetry where present.

# Execution Steps
- [x] Add OpenAI default endpoint paths and request metadata for required primitive APIs.
- [x] Add tests for Responses and Chat Completions raw JSON request/response preservation.
- [x] Add tests for Images request/response preservation and non-token billing fallback behavior.
- [x] Add tests for Audio Transcriptions multipart request handling and response usage/text preservation.
- [x] Add tests for Audio Speech binary response handling and budget fallback behavior.
- [x] Add Realtime scaffold tests that validate support status, auth, and explicit not-yet-implemented transport behavior if full realtime is deferred.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Responses acceptance: primitive OpenAI Responses payload does not pass through canonical `LlmRequest` conversion.
- Chat acceptance: primitive Chat Completions payload remains OpenAI-native.
- Image/audio acceptance: binary and multipart transport requirements are explicit and tested.
- Realtime acceptance: support status is explicit; unsupported transport fails clearly before partial dispatch.

## Task Completion Acceptance Criteria
- OpenAI required primitive APIs from the Plan have Native or explicitly scaffolded support status.
- Usage extraction covers OpenAI `usage` and cached token details where present.
- Existing canonical OpenAI Responses behavior remains unchanged.

# Dynamic Adjustments
- Current discovery: OpenAI image edits/variations can be follow-up expansion if not supported by the first transport model.
- Downstream impact: stream/realtime task depends on event/frame decisions made here.
- Recommended action: keep canonical OpenAI Responses tests separate from primitive OpenAI Responses tests.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `contract.provider.primitive_protocol`, `capability.primitive_provider.execute`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Reference: `github.com/openai/openai-go`.
- Primary files: `src/primitive.rs`, `src/gateway.rs`, `src/dispatcher.rs`, provider tests.
