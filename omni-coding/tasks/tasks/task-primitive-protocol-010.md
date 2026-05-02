---
id: task-primitive-protocol-010
title: Document dual protocol usage and finalize validation
status: done
priority: P1
tags: [primitive-protocol, docs, validation]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-001
  - task-primitive-protocol-002
  - task-primitive-protocol-003
  - task-primitive-protocol-004
  - task-primitive-protocol-005
  - task-primitive-protocol-006
  - task-primitive-protocol-007
  - task-primitive-protocol-008
  - task-primitive-protocol-009
blocks: []
---

# Background
Users need clear guidance on when to use OpenAI Responses canonical mode versus provider primitive mode. Documentation must reflect implemented support, not aspirational API coverage.

# Goal
- Document the dual protocol architecture and migration guidance.
- Add examples for canonical OpenAI Responses usage and provider primitive usage.
- Update provider support matrix, README, website docs, and bundled skill references as needed.
- Complete final validation and synchronize task/project status.

# Execution Steps
- [x] Update README architecture/usage sections with canonical versus primitive protocol guidance.
- [x] Add examples for canonical usage, OpenAI primitive usage, Anthropic primitive usage, Gemini primitive usage, and OpenAI-compatible primitive usage where implemented.
- [x] Update website docs and skill reference to describe exact provider support levels and limitations.
- [x] Run final validation commands from the Plan.
- [x] Synchronize task cards, project page, and dashboard statuses after implementation review.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Accuracy acceptance: docs distinguish implemented, scaffolded, compatible, and planned support.
- Usability acceptance: users can choose canonical or primitive mode from examples without reading source code.
- Budget acceptance: docs state that both modes use the unified token budget and explain fallback behavior.
- Validation acceptance: final commands and results are recorded in this task's execution log.

## Task Completion Acceptance Criteria
- README, website docs, examples, and task/project pages are synchronized with actual implementation status.
- `cargo fmt`, targeted primitive tests, `cargo test api_surface`, and full `cargo test` have recorded outcomes.
- No docs claim full provider SDK parity unless matching tests exist.

# Dynamic Adjustments
- Current discovery: final docs may need provider-by-provider caveats based on tasks 006 through 009.
- Downstream impact: release notes or crate docs may need updates if public API surface changes.
- Recommended action: make docs conservative and test-backed.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo fmt --check`, `cargo test primitive --tests`, `cargo test --test api_surface`, `cargo test`, `cargo check --examples`, and spec/task YAML validation pass.
- Final sync: `GatewayConfig` / `GatewayBuilder::from_config` now preserve `primitive_endpoint` as specified.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `contract.provider.primitive_protocol`, `capability.primitive_provider.execute`, `contract.public_api.surface`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Primary files: `README.md`, `examples/`, `website/docs/`, `skill/README.md`, `skill/references/`.
