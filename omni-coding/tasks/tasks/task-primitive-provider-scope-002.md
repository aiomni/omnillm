---
id: task-primitive-provider-scope-002
title: Implement P1 OpenAI primitive HTTP gaps
status: done
priority: P1
tags: [primitive-provider-scope, openai, http]
project: primitive-provider-scope-expansion
due: null
parent: null
depends_on:
  - task-primitive-provider-scope-001
blocks:
  - task-primitive-provider-scope-005
---

# Background
OpenAI primitive support covers core generation/media endpoints but P1 low-risk HTTP gaps remain for files, uploads, models, audio translations, and image edit/variation path coverage.

# Goal
- Add registry/path/auth support for OpenAI P1 HTTP gaps that fit `primitive_call`.
- Keep raw request/response bodies provider-native.
- Classify models metadata as zero-cost, uploads/files as upload/storage, and media endpoints as billable-unit or reserved-estimate fallback.

# Execution Steps
- [x] Add OpenAI primitive registry entries or path coverage for Files, Uploads, Models, Audio Translations, image edits, and image variations.
- [x] Add default path resolution and explicit-path override tests for each endpoint family.
- [x] Add request preservation tests for JSON, multipart, binary, and text bodies where applicable.
- [x] Add response preservation tests for JSON and binary/text responses.
- [x] Add budget settlement tests for zero-cost metadata, upload/storage, and billable-unit fallback.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Path acceptance: default paths match OpenAI API shapes or require explicit path when unsafe to infer.
- Payload acceptance: no OpenAI P1 request or response is converted through canonical types.
- Budget acceptance: models/files/uploads do not consume token budget unless usage/billable units are reported.

## Task Completion Acceptance Criteria
- Targeted primitive tests cover OpenAI P1 endpoints.
- Public docs distinguish implemented P1 HTTP support from realtime/binary streaming support.
- Existing OpenAI Responses canonical tests remain green.

# Dynamic Adjustments
- Current discovery: file upload and image edit/variation may require multipart fixtures.
- Downstream impact: docs task 005 depends on exact endpoint names and support levels.
- Recommended action: prefer explicit path support when provider URL shape varies by version.

# Execution Log
## 2026-05-02
- Added OpenAI P1 primitive wire formats and endpoint coverage for Files, Uploads, Models, Audio Translations, image edits, and image variations.
- Added default path coverage and budget-class tests for metadata, upload/storage, and media billable-unit fallback.
- Validation: `cargo fmt` and `cargo test primitive --tests` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; OpenAI P1 HTTP gap acceptance criteria are satisfied.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source spec tier: `support_tiers.p1_low_risk_http_gaps.OpenAi`.
- Primary files: `src/primitive.rs`, `src/dispatcher.rs`, `tests/primitive_protocol.rs`.
