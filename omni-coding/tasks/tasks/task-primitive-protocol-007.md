---
id: task-primitive-protocol-007
title: Implement Anthropic primitive API family
status: done
priority: P1
tags: [primitive-protocol, anthropic, provider-slice]
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
Anthropic primitive support should expose Messages-family provider-native payloads directly instead of converting them through OpenAI Responses canonical objects.

# Goal
- Support Anthropic Messages, streaming Messages scaffold, Count Tokens, Message Batches, and Files according to the primitive provider matrix.
- Preserve Anthropic-native request/response shape including tool use, vision, thinking, prompt cache fields, and beta/vendor fields as raw payload.
- Extract Anthropic usage and prompt-cache telemetry into the unified budget side channel.

# Execution Steps
- [x] Add Anthropic default endpoint paths, auth header, and default API version header behavior for primitive mode.
- [x] Add Messages raw JSON preservation tests for text, tools, and vision-like content blocks.
- [x] Add Count Tokens request/response handling as an estimate source where applicable.
- [x] Add Message Batches and Files support status with dispatch or explicit scaffold behavior.
- [x] Add usage extraction tests for input/output tokens and cache read/write token fields.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Messages acceptance: Anthropic-native payloads remain Anthropic-native end to end.
- Header acceptance: `x-api-key` and Anthropic version headers are applied in primitive transport only.
- Token acceptance: Count Tokens can be used as a preflight source or is explicitly scaffolded with fallback estimate behavior.
- Cache telemetry acceptance: cache read/write usage fields are preserved for budget and observability.

## Task Completion Acceptance Criteria
- Anthropic required primitive APIs have Native or explicitly scaffolded support status.
- Budget actual cost can use Anthropic token usage when present.
- Existing canonical Claude Messages bridge remains unchanged.

# Dynamic Adjustments
- Current discovery: beta headers or model-specific thinking fields may need vendor extension handling rather than typed modeling.
- Downstream impact: stream task depends on Anthropic event/frame choices.
- Recommended action: avoid normalizing Anthropic content blocks unless a Spec patch authorizes typed provider builders.

# Execution Log
## 2026-05-02
- Implemented primitive protocol support across model, registry, gateway/dispatcher runtime, unified budget telemetry, provider-family coverage, stream scaffold, docs, examples, and tests.
- Validation: `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by the primitive protocol implementation and validation.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized on 2026-05-02.

# Notes
- Source specs: `contract.provider.primitive_protocol`, `contract.prompt_cache.policy`, `capability.budget.cost`.
- Source plan: `omni-coding/plans/current/plan.primitive-protocol.md`.
- Reference: `github.com/anthropics/anthropic-sdk-typescript`.
- Primary files: `src/primitive.rs`, `src/gateway.rs`, `src/dispatcher.rs`, provider tests.
