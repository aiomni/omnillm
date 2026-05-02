---
id: task-primitive-protocol-001
title: Add canonical path guardrails
status: done
priority: P0
tags: [primitive-protocol, canonical, regression]
project: primitive-protocol
due: null
parent: null
depends_on: []
blocks:
  - task-primitive-protocol-002
  - task-primitive-protocol-004
  - task-primitive-protocol-010
---

# Background
OpenAI Responses remains OmniLLM's standard canonical protocol. The primitive protocol work must not change existing `Gateway::call`, `Gateway::stream`, generation transcoding, provider compat routing, or budget behavior for current users.

# Goal
- Establish regression coverage that freezes current canonical behavior before primitive implementation begins.
- Prove existing OpenAI Responses, OpenAI Chat Completions, Claude Messages, Gemini GenerateContent, and OpenAI-compatible flows remain reachable through the current canonical path.
- Capture baseline budget settlement behavior for canonical call and stream paths.

# Execution Steps
- [x] Review existing generation, gateway, stream budget, provider registry, and API surface tests for canonical coverage gaps.
- [x] Add or extend tests only where current canonical behavior is not already protected.
- [x] Verify current public API construction does not require any primitive configuration.
- [x] Record any existing gaps that should be handled before provider primitive work starts.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Canonical API acceptance: `Gateway::call` and `Gateway::stream` remain callable with existing `LlmRequest` inputs.
- Provider coverage acceptance: OpenAI Responses, OpenAI Chat Completions, Claude Messages, and Gemini GenerateContent canonical test coverage exists.
- Budget acceptance: canonical reserve/refund/settlement behavior has targeted coverage or an explicit existing-test reference.
- Compatibility acceptance: no primitive type is required by existing user-facing constructors.

## Task Completion Acceptance Criteria
- Canonical regression tests pass before primitive behavior is introduced.
- Any uncovered canonical behavior is tracked as a follow-up before downstream tasks modify shared gateway code.
- No primitive implementation is added in this task.

# Dynamic Adjustments
- Current discovery: worktree may contain interrupted primitive source drafts; decide whether to clean or reuse them before implementation tasks start.
- Downstream impact: failures here block all implementation tasks that touch `Gateway`, `Dispatcher`, `pricing`, or public reexports.
- Recommended action: keep this task focused on preserving existing behavior only.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `contract.provider.primitive_protocol`, `capability.gateway.execute`, `capability.budget.cost`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Primary files: `tests/generation_matrix.rs`, `tests/gateway_stream_budget.rs`, `tests/api_surface.rs`.
