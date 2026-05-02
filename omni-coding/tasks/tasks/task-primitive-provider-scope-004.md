---
id: task-primitive-provider-scope-004
title: Implement P1 Gemini metadata operations and file gaps
status: todo
priority: P1
tags: [primitive-provider-scope, gemini, http]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-001
blocks:
  - task-primitive-provider-scope-005
---

# Background
Gemini primitive support covers generate, stream, count tokens, embeddings, files, and caches basics; P1 requires Models, read-only Operations, and path hardening.

# Goal
- Add Gemini Models and read-only Operations primitive support.
- Harden Files and Caches path coverage.
- Keep Gemini API key auth, raw payloads, and usage metadata handling provider-native.

# Execution Steps
- [ ] Add Gemini Models registry/path coverage.
- [ ] Add read-only Operations path coverage for get/list where applicable.
- [ ] Audit Gemini Files and Caches default path resolution and explicit path behavior.
- [ ] Add auth/query/header tests for Gemini and Vertex-compatible endpoint forms.
- [ ] Add budget tests for zero-cost metadata and cache/file operations.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Path acceptance: model-scoped and resource-scoped Gemini paths resolve predictably or require explicit path.
- Auth acceptance: Gemini primitive auth remains isolated from canonical Gemini generation path.
- Budget acceptance: models/operations metadata settle zero unless usage exists.

## Task Completion Acceptance Criteria
- Gemini P1 primitive tests cover Models, Operations, Files, and Caches hardening.
- Docs do not claim Gemini tunings, file search stores, Imagen/Veo, or hosted RAG administration support.
- Existing Gemini generate/stream primitive tests remain green.

# Dynamic Adjustments
- Current discovery: Gemini Operations are shared by multiple async APIs and may become a P2 dependency.
- Downstream impact: P2 batch lifecycle depends on reliable Operations polling semantics.
- Recommended action: implement read-only Operations narrowly before adding batch create flows.

# Execution Log
- Not started.

# Review
- Review status: pending.

# Notes
- Source spec tier: `support_tiers.p1_low_risk_http_gaps.Gemini`.
- Primary files: `src/primitive.rs`, `src/dispatcher.rs`, `tests/primitive_protocol.rs`.
