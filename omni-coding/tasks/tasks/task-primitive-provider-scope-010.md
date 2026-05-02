---
id: task-primitive-provider-scope-010
title: Finalize primitive expansion docs validation and support claims
status: done
priority: P1
tags: [primitive-provider-scope, docs, validation]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-005
  - task-primitive-provider-scope-007
  - task-primitive-provider-scope-008
  - task-primitive-provider-scope-009
blocks: []
---

# Background
After P1/P2/P3 implementation, public docs and task state must reflect exact support without implying full SDK parity.

# Goal
- Audit primitive provider support claims across README, website docs, skill reference, examples, specs, and task dashboard.
- Record final validation results.
- Ensure deferred APIs remain documented as deferred.

# Execution Steps
- [x] Audit support matrix against implemented registry and tests.
- [x] Update README, website docs, skill reference, and examples for final P1/P2/P3 state.
- [x] Update specs if support levels changed during implementation.
- [x] Run final validation commands from the expansion plan.
- [x] Synchronize project page, task cards, and dashboard statuses.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Accuracy acceptance: every support claim maps to a passing test or explicit Planned/Deferred state.
- Validation acceptance: final command list and outcomes are recorded.
- Scope acceptance: docs state OmniLLM is not a provider admin SDK or full SDK parity layer.

## Task Completion Acceptance Criteria
- `cargo fmt`, `cargo fmt --check`, `cargo test primitive --tests`, `cargo test --test api_surface`, `cargo test`, and `cargo check --examples` pass.
- Spec/task YAML validation passes.
- Project page and dashboard show no stale P1/P2/P3 statuses.

# Dynamic Adjustments
- Current discovery: docs may need provider-by-provider caveats for beta APIs.
- Downstream impact: release notes may need support-tier summary if public API changes.
- Recommended action: keep support wording conservative and test-backed.

# Execution Log
- Audited support claims across README, website docs, skill reference, examples, current Specs, project pages, and task dashboard.
- Updated public wording to state exact implemented P1/P2/P3 scope: P1 HTTP gaps, P2 async batch-style lifecycle, P3 OpenAI Audio Speech binary chunks, OpenAI Realtime WebSocket, and Gemini Live WebSocket.
- Recorded WebRTC as planned/not implemented and kept deferred admin, billing, fine-tuning, eval, tuning, managed-agent, hosted RAG, webhook, and SDK-helper parity out of supported scope.
- Added `examples/primitive_transport_modes.rs` as a compile-only P3 transport construction example.
- Validation passed: `cargo fmt`, `cargo fmt --check`, `cargo test primitive --tests`, `cargo test --test api_surface`, `cargo test`, `cargo check --examples`, and current Spec YAML and task frontmatter validation.

# Review
- Review status: done.
- Task can be marked done: all public support claims are conservative and map to registry/test-backed implementation or explicit Planned/Deferred state.
- No new follow-up task is required for this expansion slice; WebRTC remains planned and should only become a task after a current Spec promotes its transport tests.

# Notes
- Source plan: `omni-coding/plans/current/plan.primitive-provider-scope-expansion.md`.
- Primary files: `README.md`, `website/docs/`, `skill/references/`, `examples/`, `omni-coding/tasks/`.
