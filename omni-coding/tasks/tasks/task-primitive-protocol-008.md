---
id: task-primitive-protocol-008
title: Implement Gemini and Vertex primitive API family
status: done
priority: P1
tags: [primitive-protocol, gemini, vertex, provider-slice]
project: primitive-protocol
due: null
parent: null
depends_on:
  - task-primitive-protocol-003
  - task-primitive-protocol-004
  - task-primitive-protocol-005
blocks:
  - task-primitive-protocol-009
  - task-primitive-protocol-010
---

# Background
Gemini and Vertex primitive support should follow the Google GenAI API family while preserving provider-native payloads and usage metadata.

# Goal
- Support Gemini GenerateContent, streamGenerateContent scaffold, CountTokens, EmbedContent, Live scaffold, Files, and Caches.
- Represent Vertex-specific routing as compatible support without changing Gemini Developer API behavior.
- Extract Gemini `usageMetadata` into budget telemetry.

# Execution Steps
- [x] Add Gemini default endpoint path templates for model-scoped actions.
- [x] Add GenerateContent raw request/response preservation tests.
- [x] Add CountTokens as a provider-native preflight estimate source where practical.
- [x] Add EmbedContent, Files, and Caches support status and dispatch/scaffold behavior.
- [x] Add Live API scaffold with explicit transport capability status.
- [x] Add usage extraction tests for prompt, candidates, and total token counts.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- GenerateContent acceptance: Gemini-native `contents`, `systemInstruction`, `tools`, `generationConfig`, and safety settings remain raw provider payload.
- CountTokens acceptance: CountTokens can inform estimates or has explicit fallback behavior.
- Vertex acceptance: Vertex routing requirements are not hidden inside Gemini Developer API defaults.
- Usage acceptance: `usageMetadata` projects into budget telemetry without mutating response body.

## Task Completion Acceptance Criteria
- Gemini required primitive APIs have Native or explicitly scaffolded support status.
- Vertex compatibility is represented without claiming unsupported endpoints as enabled.
- Existing canonical Gemini GenerateContent bridge remains unchanged.

# Dynamic Adjustments
- Current discovery: Live API and Vertex auth/routing may need feature gates or custom endpoint configuration before full support.
- Downstream impact: stream/realtime task depends on Gemini stream and Live event decisions.
- Recommended action: treat Vertex as compatible routing unless a separate Vertex Spec is created.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `contract.provider.primitive_protocol`, `capability.primitive_provider.execute`, `capability.budget.cost`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Reference: `github.com/googleapis/go-genai`.
- Primary files: `src/primitive.rs`, `src/gateway.rs`, `src/dispatcher.rs`, provider tests.
