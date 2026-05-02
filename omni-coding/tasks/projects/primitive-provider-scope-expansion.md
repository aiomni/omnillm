# Primitive Provider Scope Expansion

## Scope
- 按 `contract.primitive_provider.scope` 扩展 provider-native primitive API，而不是追求完整 provider SDK parity。
- 保持 OmniLLM 定位为 LLM gateway、budget tracker 和 protocol bridge。
- 优先补 P1 low-risk HTTP gaps，再补 P2 async job lifecycle，最后补 P3 binary/realtime transports。
- 每个新增支持声明必须有 registry、path/auth、payload preservation、error mapping、budget settlement 和 docs/support matrix 验收。

## Out Of Scope
- Provider admin、billing、audit、webhook、organization/project/user/key 管理。
- Fine-tuning、evals、graders、tunings、managed-agent platform、hosted RAG/vector-store administration。
- SDK convenience helpers，例如 poller wrapper、chat accumulator、typed parser helper。
- 未经 current Spec 提升的 deferred API。

## Constraints
- Source of truth: `contract.primitive_provider.scope`、`contract.provider.primitive_protocol`、`capability.primitive_provider.execute`、`capability.budget.cost`。
- OpenAI Responses 仍是 canonical standard protocol。
- Primitive path 不做 canonical request/response conversion。
- Metadata/read-only endpoints 必须显式归类为 zero-cost 或 provider-reported usage。
- Async jobs、binary chunk streaming、realtime session 不得复用普通 non-stream `primitive_call` 的隐式生命周期。

## Current State
- Todo: scope guardrails and registry vocabulary hardening.
- Todo: P1 OpenAI low-risk HTTP gaps.
- Todo: P1 Anthropic metadata/files hardening.
- Todo: P1 Gemini metadata/operations/files/caches hardening.
- Todo: P1 docs/examples/support matrix sync.
- Todo: P2 primitive async job lifecycle design and runtime shape.
- Todo: P2 cross-provider batch lifecycle implementation.
- Todo: P3 binary chunk streaming transport.
- Todo: P3 realtime session transport.
- Todo: final docs, validation, and support claim audit.

## Milestones
- Milestone 1: Scope guardrails — tasks 001.
- Milestone 2: P1 HTTP gaps — tasks 002 through 005.
- Milestone 3: P2 async jobs — tasks 006 and 007.
- Milestone 4: P3 transports — tasks 008 and 009.
- Milestone 5: final support claim audit — task 010.

## Acceptance Signals
- Deferred APIs fail registry/guardrail checks unless a current Spec explicitly promotes them.
- P1 HTTP endpoints preserve raw payloads and have explicit zero-cost, token, billable-unit, or upload/storage budget class.
- P2 batch APIs expose create/get/list/cancel/result lifecycle without double-settling budget.
- P3 binary/realtime transports settle exactly once on close, EOF, provider error, and cancellation.
- README, website docs, skill reference, and examples only claim test-backed support.

## Final Acceptance Criteria
- `cargo fmt`, `cargo fmt --check`, `cargo test primitive --tests`, `cargo test --test api_surface`, `cargo test`, and `cargo check --examples` pass.
- Spec/task YAML validation passes.
- Project page, task cards, and dashboard are synchronized.
- Support matrix distinguishes Implemented, Compatible, Scaffolded, Planned, and Deferred.
