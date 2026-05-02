---
id: task-primitive-protocol-005
title: Add unified primitive budget projection
status: done
priority: P0
tags: [primitive-protocol, budget, pricing]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-002
  - task-primitive-protocol-004
blocks:
  - task-primitive-protocol-006
  - task-primitive-protocol-007
  - task-primitive-protocol-008
  - task-primitive-protocol-009
  - task-primitive-protocol-010
---

# Background
Primitive protocol support must reuse the same budget calculation and settlement model as the canonical Gateway path while avoiding payload mutation.

# Goal
- Estimate primitive request cost using provider token-count hints, provider-specific estimators, existing heuristics, or conservative fallback.
- Extract provider usage into side-channel telemetry without modifying primitive response body.
- Settle primitive budget exactly once across success, error, local rejection, cancellation, stream EOF, and realtime close paths.

# Execution Steps
- [x] Add primitive request token estimate helpers using existing heuristics as fallback.
- [x] Add provider usage extraction for OpenAI usage, Anthropic usage, Gemini `usageMetadata`, and OpenAI-compatible usage.
- [x] Add primitive actual-cost calculation through existing pricing model when `TokenUsage` is available.
- [x] Define fallback behavior when only billable units or no usage telemetry is present.
- [x] Add tests for reserve, refund, settle-down, settle-up, cancellation, provider error, and local RPM rejection.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Shared tracker acceptance: primitive calls use the existing `BudgetTracker` type and do not create another budget subsystem.
- Estimate acceptance: preflight estimate never assumes cache hits or provider discounts before response telemetry exists.
- Settlement acceptance: primitive execution calls settle exactly once per reserved execution path.
- Preservation acceptance: usage extraction does not mutate returned primitive payload.

## Task Completion Acceptance Criteria
- Budget tests prove canonical and primitive paths share the same observable budget used/remaining behavior.
- Provider usage parsing tests cover OpenAI, Anthropic, Gemini, and OpenAI-compatible payload shapes.
- Missing usage falls back to reserved estimate on success and full refund on pre-response failure.

# Dynamic Adjustments
- Current discovery: provider-specific image/audio/realtime billing may require billable-unit support beyond token usage.
- Downstream impact: provider family tasks must add usage fixtures matching this task's telemetry model.
- Recommended action: if pricing tables need new hard truth, patch `capability.budget.cost` before implementation.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.
- Follow-up hardening: added provider-error refund, local RPM refund, and SSE cancellation refund tests; `cargo fmt --check`, `cargo check --examples`, and spec/task YAML validation pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `capability.budget.cost`, `capability.primitive_provider.execute`, `contract.prompt_cache.policy`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Primary files: `src/pricing.rs`, `src/gateway.rs`, `src/primitive.rs`, `src/budget/tracker.rs`.
