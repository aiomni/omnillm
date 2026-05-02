---
id: task-primitive-provider-scope-003
title: Implement P1 Anthropic metadata and files gaps
status: todo
priority: P1
tags: [primitive-provider-scope, anthropic, http]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-001
blocks:
  - task-primitive-provider-scope-005
---

# Background
Anthropic primitive support covers Messages, Count Tokens, Message Batches, and Files basics; P1 requires Models and Files path hardening.

# Goal
- Add or harden Anthropic Models and Files primitive support.
- Preserve Anthropic-native headers and raw payloads.
- Classify Models as zero-cost metadata and Files as upload/storage or zero-cost metadata by operation.

# Execution Steps
- [ ] Add Anthropic Models registry/path coverage.
- [ ] Audit Anthropic Files create/get/list/delete/download path coverage and mark unsupported operations explicitly.
- [ ] Add auth/default header tests for `x-api-key` and `anthropic-version` behavior.
- [ ] Add payload preservation tests for file upload and file metadata responses.
- [ ] Add budget tests for zero-cost metadata and upload/storage behavior.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Header acceptance: Anthropic primitive calls keep required provider-native headers without affecting canonical calls.
- Path acceptance: Files and Models operations either resolve default paths or require explicit path with clear protocol error.
- Budget acceptance: metadata operations settle zero unless provider usage appears.

## Task Completion Acceptance Criteria
- Anthropic P1 primitive tests cover models and files hardening.
- Docs describe Anthropic support without claiming managed-agent platform support.
- No deferred Anthropic Skills, Agents, Sessions, Environments, Admin, or Usage APIs enter registry.

# Dynamic Adjustments
- Current discovery: file operation shapes may differ across beta headers.
- Downstream impact: docs task 005 depends on final support level names.
- Recommended action: keep beta-only operations Planned unless tests include required headers.

# Execution Log
- Not started.

# Review
- Review status: pending.

# Notes
- Source spec tier: `support_tiers.p1_low_risk_http_gaps.Anthropic`.
- Primary files: `src/primitive.rs`, `src/dispatcher.rs`, `tests/primitive_protocol.rs`.
