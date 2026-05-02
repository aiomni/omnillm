# Tasks Dashboard

Last updated: 2026-05-02

Guide: see `omni-coding/tasks/README.md`
Projects:
- `omni-coding/tasks/projects/primitive-provider-scope-expansion.md`
- `omni-coding/tasks/projects/primitive-protocol.md`
- `omni-coding/tasks/projects/prompt-cache.md`
Plans:
- `omni-coding/plans/current/plan.primitive-provider-scope-expansion.md`
- `omni-coding/plans/current/plan.primitive-protocol.md`
- `omni-coding/plans/current/plan.prompt-cache.md`

## Doing
- No active tasks.

## Todo
- `task-primitive-provider-scope-007`: Implement P2 batch lifecycle providers
- `task-primitive-provider-scope-008`: Implement binary chunk streaming transport
- `task-primitive-provider-scope-009`: Implement realtime session transports
- `task-primitive-provider-scope-010`: Finalize primitive expansion docs validation and support claims

## Blocked
- No blocked tasks.

## Done
- `task-primitive-provider-scope-006`: Define primitive async job lifecycle
- `task-primitive-provider-scope-005`: Sync P1 primitive support docs and examples
- `task-primitive-provider-scope-004`: Implement P1 Gemini metadata operations and file gaps
- `task-primitive-provider-scope-003`: Implement P1 Anthropic metadata and files gaps
- `task-primitive-provider-scope-002`: Implement P1 OpenAI primitive HTTP gaps
- `task-primitive-provider-scope-001`: Add primitive scope guardrails and registry vocabulary
- `task-primitive-protocol-001`: Add canonical path guardrails
- `task-primitive-protocol-002`: Define primitive public model and mode boundary
- `task-primitive-protocol-003`: Add primitive provider registry and support matrix
- `task-primitive-protocol-004`: Implement primitive non-stream execution path
- `task-primitive-protocol-005`: Add unified primitive budget projection
- `task-primitive-protocol-006`: Implement OpenAI primitive API family
- `task-primitive-protocol-007`: Implement Anthropic primitive API family
- `task-primitive-protocol-008`: Implement Gemini and Vertex primitive API family
- `task-primitive-protocol-009`: Add primitive streaming and realtime scaffold
- `task-primitive-protocol-010`: Document dual protocol usage and finalize validation
- `task-prompt-cache-001`: Define prompt cache canonical types
- `task-prompt-cache-002`: Parse prompt cache usage telemetry
- `task-prompt-cache-003`: Emit OpenAI prompt cache fields
- `task-prompt-cache-004`: Emit Claude cache_control breakpoints
- `task-prompt-cache-005`: Apply prompt cache bridge semantics
- `task-prompt-cache-006`: Add prompt prefix builder
- `task-prompt-cache-007`: Add cache-aware pricing
- `task-prompt-cache-008`: Update prompt cache documentation

## Critical Path
- Primitive provider scope expansion: scope guardrails and P1 HTTP gaps done; todo through P2 async jobs, P3 transports, and final support claim audit.
- Primitive protocol: completed through canonical guardrails, primitive model, registry, non-stream execution, unified budget projection, provider family slices, SSE stream scaffold, realtime scaffold, docs, examples, config/spec sync, and validation.
- Prompt cache: completed.

## Milestones
- Primitive Provider Expansion Milestone 1: scope guardrails — done.
- Primitive Provider Expansion Milestone 2: P1 HTTP gaps — done.
- Primitive Provider Expansion Milestone 3: P2 async jobs — todo.
- Primitive Provider Expansion Milestone 4: P3 transports — todo.
- Primitive Provider Expansion Milestone 5: final support claim audit — todo.
- Primitive Milestone 1: canonical guardrails and public mode boundary — done.
- Primitive Milestone 2: primitive registry and non-stream execution — done.
- Primitive Milestone 3: unified budget and usage projection — done.
- Primitive Milestone 4: provider API family coverage — done.
- Primitive Milestone 5: streaming/realtime and release readiness — done.
- Prompt cache milestones — done.

## Final Acceptance
- OpenAI Responses remains the default standard protocol and existing Gateway calls stay backward compatible.
- Provider primitive calls can send and receive provider-native payloads without canonical conversion.
- OpenAI, Anthropic, Gemini/Vertex, OpenAI-compatible, Bedrock, and Custom HTTP support levels are explicit.
- Canonical and primitive paths use one token budget system with reserve/refund/settle-once guarantees.
- Documentation and examples explain both protocol modes and only claim test-backed provider support.
- Validation completed with `cargo fmt`, `cargo test primitive --tests`, `cargo test --test api_surface`, and `cargo test`.
