---
id: task-primitive-protocol-003
title: Add primitive provider registry and support matrix
status: done
priority: P0
tags: [primitive-protocol, registry, provider-support]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-002
blocks:
  - task-primitive-protocol-004
  - task-primitive-protocol-006
  - task-primitive-protocol-007
  - task-primitive-protocol-008
  - task-primitive-protocol-009
  - task-primitive-protocol-010
---

# Background
Primitive dispatch must be gated by provider, endpoint, wire format, and stream mode before key acquisition, RPM acquisition, budget reservation, or network dispatch.

# Goal
- Add a primitive provider registry separate from the existing canonical provider registry.
- Represent Native, Compatible, and Planned support for OpenAI, Azure OpenAI, Anthropic, Gemini, Vertex AI, Bedrock, OpenAI-Compatible, and Custom HTTP.
- Encode initial API coverage from the Plan's provider API coverage matrix.

# Execution Steps
- [x] Define primitive registry descriptor and endpoint support records.
- [x] Add lookup APIs for provider, endpoint, wire format, and stream mode support.
- [x] Add embedded support data or code-backed defaults for the initial provider matrix.
- [x] Ensure Planned endpoints are not enabled.
- [x] Add tests for missing provider, missing endpoint, unsupported wire format, unsupported stream mode, and planned support rejection.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Registry acceptance: primitive support lookup returns false for unknown provider or endpoint.
- Safety acceptance: Planned support is not dispatchable.
- Separation acceptance: primitive registry does not change existing `ProviderRegistry` behavior.
- Coverage acceptance: OpenAI, Anthropic, Gemini, and OpenAI-compatible required APIs are represented with explicit support status.

## Task Completion Acceptance Criteria
- Unsupported primitive endpoint checks can run before key/RPM/budget/network operations.
- Registry tests cover Native, Compatible, Planned, unknown, and custom provider behavior.
- Existing canonical provider registry tests continue to pass.

# Dynamic Adjustments
- Current discovery: provider matrix may need to move to `support/` if static data grows beyond readable code-backed defaults.
- Downstream impact: tasks 004, 006, 007, 008, and 009 depend on registry names and support semantics.
- Recommended action: keep canonical and primitive registries separate unless a later Spec explicitly merges them.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `contract.provider.primitive_protocol`, `contract.provider.registry`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Primary files: `src/provider_registry.rs`, `src/primitive.rs`, `support/provider_support_matrix.json`.
