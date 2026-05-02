# Primitive Provider Scope Expansion Plan

status: current
version: 1
source_specs:
  - contract.primitive_provider.scope
  - contract.provider.primitive_protocol
  - capability.primitive_provider.execute
  - capability.budget.cost

## Goal

Expand primitive provider support where it strengthens OmniLLM's role as an LLM gateway, budget tracker, and provider-native protocol bridge, without turning the crate into a full provider admin SDK.

The expansion should close high-value primitive gaps for OpenAI, Anthropic, Gemini, and OpenAI-compatible providers while keeping OpenAI Responses as the canonical standard protocol and keeping budget settlement unified.

## Non-Goals

- Do not pursue full provider SDK parity.
- Do not add admin, billing, audit, webhook, organization/project/user/key management, managed-agent platform, hosted RAG control-plane, fine-tuning, eval, grader, or tuning APIs unless a new current Spec explicitly promotes them.
- Do not hide async job, realtime, WebSocket, WebRTC, or binary chunk behavior inside the simple non-streaming `primitive_call` lifecycle.
- Do not add a second budget subsystem for metadata, batch, realtime, or media endpoints.

## Phase Order

1. Scope guardrails and registry vocabulary
   - Keep `contract.primitive_provider.scope` as the expansion source of truth.
   - Add future endpoint/wire-format names only after assigning a support tier and budget class.
   - Acceptance: deferred APIs cannot appear as Native/Compatible without a scope promotion.
2. P1 low-risk HTTP gaps
   - Add zero-cost or billable-unit-classified HTTP primitive gaps that fit `primitive_call`.
   - Target OpenAI Files, Uploads, Models, Audio Translations, and image edit/variation path coverage.
   - Target Anthropic Models and Files path coverage hardening.
   - Target Gemini Models, read-only Operations, Files path coverage, and Caches path coverage.
   - Acceptance: each endpoint preserves raw payloads, has auth/path tests, and settles zero or provider-reported budget correctly.
3. P2 async job lifecycle
   - Define an explicit primitive async-job lifecycle before broadening Batch support.
   - Cover OpenAI Batches, Anthropic Message Batches lifecycle, Gemini Batches, and Gemini Operations polling.
   - Acceptance: create/get/list/cancel/results paths have clear budget and cancellation semantics.
4. P3 transport expansion
   - Implement binary chunk streaming separately from SSE.
   - Implement realtime session transport separately from HTTP calls.
   - Target OpenAI Audio Speech binary chunks, OpenAI Realtime WebSocket/WebRTC, and Gemini Live WebSocket.
   - Acceptance: stream/session close, cancellation, partial usage, and provider error paths settle exactly once.
5. Documentation and support claims
   - Update README, website docs, skill reference, and examples after each tier.
   - Keep support tables conservative: Implemented, Compatible, Scaffolded, Planned, or Deferred.
   - Acceptance: docs never imply full provider SDK parity.

## Technical Approach

- Use additive registry slices. Existing canonical OpenAI Responses behavior and existing primitive HTTP/SSE behavior must remain unchanged.
- Model metadata and upload endpoints as explicit zero-cost or upload/storage budget classes unless provider usage telemetry says otherwise.
- Model batch APIs as async jobs rather than ordinary generation calls once polling/result retrieval enters scope.
- Model binary chunk streaming as a `PrimitiveStreamEvent::BinaryChunk` transport path, not as base64 text SSE.
- Model realtime sessions as lifecycle-managed transports with open/send/receive/close/cancel semantics and accumulated usage settlement.

## Ordering Rationale

| Phase | Why First Or Later |
| --- | --- |
| Scope guardrails | Prevents primitive support from expanding into unbounded SDK parity. |
| P1 HTTP gaps | Low transport risk and improves provider-native completeness quickly. |
| P2 async jobs | Requires lifecycle and polling semantics that should not leak into simple calls. |
| P3 transports | WebSocket/WebRTC and binary streaming have the highest cancellation and budget risk. |
| Docs after each tier | Keeps public claims aligned with tested support. |

## Risks And Mitigations

- Risk: endpoint count grows faster than validation. Mitigation: require registry, path, payload preservation, error, and budget tests per support claim.
- Risk: zero-cost metadata endpoints distort token budget reporting. Mitigation: classify metadata/upload endpoints explicitly and only charge provider-reported usage or billable units.
- Risk: async batch lifecycle double-settles budget. Mitigation: introduce job lifecycle settlement rules before implementing broad batch polling/results.
- Risk: realtime and binary streams leak reservations on cancellation. Mitigation: require cancellation and partial usage tests before enabling support.
- Risk: hosted RAG or admin surfaces dilute project positioning. Mitigation: keep them deferred unless a new current Spec promotes them.

## Validation

- `cargo fmt`
- `cargo fmt --check`
- `cargo test primitive --tests`
- `cargo test --test api_surface`
- `cargo test`
- `cargo check --examples`
- Spec/task YAML validation after current Spec or task updates.

## Task Candidates

- Add primitive scope guardrail tests for deferred APIs.
- Add P1 OpenAI Files/Uploads/Models/Audio Translations path support.
- Add P1 Anthropic Models and Files path hardening.
- Add P1 Gemini Models/Operations/Files/Caches path hardening.
- Define primitive async job lifecycle and budget semantics.
- Add P2 Batch lifecycle support across OpenAI, Anthropic, and Gemini.
- Add binary chunk streaming transport and tests.
- Add realtime session transport and tests for OpenAI Realtime and Gemini Live.
- Update docs/examples after each tier.
