---
id: task-primitive-provider-scope-001
title: Add primitive scope guardrails and registry vocabulary
status: todo
priority: P0
tags: [primitive-provider-scope, registry, guardrails]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on: []
blocks:
  - task-primitive-provider-scope-002
  - task-primitive-provider-scope-003
  - task-primitive-provider-scope-004
  - task-primitive-provider-scope-006
  - task-primitive-provider-scope-008
  - task-primitive-provider-scope-009
---

# Background
Primitive provider expansion must stay inside `contract.primitive_provider.scope` and avoid drifting into full provider SDK parity.

# Goal
- Encode scope tiers and budget classes in registry-facing structures or tests.
- Make deferred APIs fail before dispatch unless promoted by current Spec.
- Preserve existing canonical and primitive behavior while adding guardrails.

# Execution Steps
- [ ] Audit current `PrimitiveEndpointKind` and `ProviderPrimitiveWireFormat` against `contract.primitive_provider.scope`.
- [ ] Add or update registry tests for P0, P1, P2, P3, and deferred support classifications.
- [ ] Add guardrail tests proving deferred APIs are not Native/Compatible by default.
- [ ] Add budget-class fixtures for token, billable-unit, zero-cost metadata, and upload/storage endpoint classes.
- [ ] Update public support matrix docs only if registry semantics change.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Scope acceptance: each primitive registry support claim maps to a scope tier.
- Deferred acceptance: admin, billing, fine-tuning, evals, tunings, managed-agent, and hosted RAG APIs remain deferred unless promoted by current Spec.
- Budget acceptance: every endpoint family has an explicit budget class before runtime support is claimed.

## Task Completion Acceptance Criteria
- Tests fail if a deferred API enters Native/Compatible support without a spec-backed scope promotion.
- Existing primitive tests still pass unchanged or with strictly additive assertions.
- Project page and dashboard are synchronized after completion.

# Dynamic Adjustments
- Current discovery: some enum names may already exist for future use; registry support level, not enum existence, controls support claims.
- Downstream impact: all P1/P2/P3 tasks depend on the support-tier vocabulary established here.
- Recommended action: avoid renaming existing public enums unless unavoidable.

# Execution Log
- Not started.

# Review
- Review status: pending.

# Notes
- Source specs: `contract.primitive_provider.scope`, `contract.provider.primitive_protocol`.
- Source plan: `omni-coding/plans/current/plan.primitive-provider-scope-expansion.md`.
- Primary files: `src/primitive.rs`, `tests/primitive_protocol.rs`, docs support matrix.
