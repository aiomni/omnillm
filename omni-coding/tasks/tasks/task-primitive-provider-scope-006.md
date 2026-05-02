---
id: task-primitive-provider-scope-006
title: Define primitive async job lifecycle
status: todo
priority: P0
tags: [primitive-provider-scope, async-jobs, budget]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-001
  - task-primitive-provider-scope-005
blocks:
  - task-primitive-provider-scope-007
  - task-primitive-provider-scope-010
---

# Background
Batch APIs are async job lifecycles and should not be hidden inside ordinary one-shot `primitive_call` semantics.

# Goal
- Define primitive async job request/response/event model before implementing broad Batch support.
- Define budget reservation and settlement for create, poll, cancel, result retrieval, and provider usage discovery.
- Keep async job lifecycle additive and separate from canonical calls.

# Execution Steps
- [ ] Draft current Spec patch for primitive async job lifecycle if existing specs are insufficient.
- [ ] Define public or crate-internal async job types and operation states needed by batch providers.
- [ ] Define budget semantics for job create, metadata polling, cancellation, and result usage settlement.
- [ ] Define key/RPM behavior for create, poll, cancel, and result retrieval calls.
- [ ] Add API-surface tests or compile tests for the chosen lifecycle boundary.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Spec acceptance: lifecycle truth is in current specs, not only in code or plan.
- Budget acceptance: job lifecycle cannot double-settle reserved budget.
- API acceptance: ordinary `primitive_call` remains valid for simple HTTP calls and is not overloaded with job state.

## Task Completion Acceptance Criteria
- Async job lifecycle is ready for OpenAI, Anthropic, and Gemini batch implementation.
- Tests cover cancellation and repeated polling semantics at the model boundary.
- Docs identify batch support as async lifecycle, not normal primitive call parity.

# Dynamic Adjustments
- Current discovery: providers report batch usage at different lifecycle points.
- Downstream impact: task 007 depends on the final settlement model.
- Recommended action: prefer conservative zero-cost polling and settle only when provider usage is observed.

# Execution Log
- Not started.

# Review
- Review status: pending.

# Notes
- Source spec tier: `support_tiers.p2_async_job_lifecycle`.
- Primary files: specs, `src/primitive.rs`, `src/gateway.rs`, `tests/primitive_protocol.rs`.
