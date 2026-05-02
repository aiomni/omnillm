---
id: task-primitive-provider-scope-005
title: Sync P1 primitive support docs and examples
status: todo
priority: P1
tags: [primitive-provider-scope, docs, examples]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-002
  - task-primitive-provider-scope-003
  - task-primitive-provider-scope-004
blocks:
  - task-primitive-provider-scope-006
  - task-primitive-provider-scope-007
  - task-primitive-provider-scope-010
---

# Background
After P1 HTTP gaps land, docs must reflect exact implemented support without implying full provider SDK parity.

# Goal
- Update support matrix, examples, and skill references for P1 primitive HTTP support.
- Explain zero-cost metadata, upload/storage, token, and billable-unit budget classes.
- Keep deferred APIs visibly out of scope.

# Execution Steps
- [ ] Update README primitive support table and examples.
- [ ] Update website English and Chinese docs.
- [ ] Update bundled skill reference API surface notes.
- [ ] Add or refresh examples for one OpenAI P1 endpoint, one Anthropic P1 endpoint, and one Gemini P1 endpoint.
- [ ] Record validation commands and support caveats in project/task pages.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Accuracy acceptance: docs distinguish Implemented, Compatible, Scaffolded, Planned, and Deferred.
- Budget acceptance: docs explain when metadata/upload endpoints settle zero.
- Scope acceptance: docs explicitly say OmniLLM is not full provider SDK parity.

## Task Completion Acceptance Criteria
- Docs and examples match tested P1 support.
- No deferred API appears as supported in docs.
- Project page and dashboard reflect P1 completion state.

# Dynamic Adjustments
- Current discovery: examples should avoid live-network requirements unless clearly marked.
- Downstream impact: P2 tasks depend on clear support matrix language.
- Recommended action: keep examples minimal and provider-native.

# Execution Log
- Not started.

# Review
- Review status: pending.

# Notes
- Source plan phase: P1 low-risk HTTP gaps and documentation.
- Primary files: `README.md`, `website/docs/`, `skill/references/`, `examples/`.
