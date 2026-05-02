# Primitive Protocol Architecture Implementation Plan

status: current
version: 2
source_specs:
  - contract.provider.primitive_protocol
  - capability.primitive_provider.execute
  - capability.budget.cost
  - capability.gateway.execute
  - contract.generation.transcoding
  - contract.provider.registry

## Goal

Deliver a dual-protocol OmniLLM architecture:

1. Keep OpenAI Responses as the standard canonical protocol. Existing `Gateway::call` / `Gateway::stream` style usage must continue to work for all currently supported LLM providers through the existing canonical conversion and provider dispatch path.
2. Add provider primitive protocol support. Callers can send provider-native requests directly to each LLM provider without converting through `LlmRequest`, `LlmResponse`, `ApiRequest`, or `ApiResponse`.
3. Keep token budget unified. Both OpenAI Responses canonical calls and provider primitive calls must use the same budget tracker, cost estimation policy, reservation order, settlement behavior, cancellation behavior, and key/RPM protection model.

## Non-Goals

- Do not replace the OpenAI Responses canonical protocol.
- Do not remove existing provider transcoding or compatibility behavior.
- Do not force primitive provider APIs into a lowest-common-denominator schema.
- Do not create a second budget or accounting subsystem.
- Do not claim full SDK parity until each provider API family has explicit coverage and validation.

## Requirement Traceability

| User Requirement | Plan Coverage | Acceptance Signal |
| --- | --- | --- |
| 保留 OpenAI Responses 作为标准协议 | Phases 1, 2, and 8 explicitly preserve canonical gateway behavior | Existing canonical tests pass unchanged; no public `Gateway::call` / `Gateway::stream` breaking changes |
| 现有调用方式可调用各个 LLM Provider | Canonical path remains default and keeps provider transcoding/compat routing | Existing OpenAI, OpenAI-compatible, Claude Messages, and Gemini GenerateContent matrix stays green |
| 支持 primitive Protocol | Phases 3-7 add provider-native endpoint config, request/response transport, registry, and dispatch | Primitive calls can send raw OpenAI, Anthropic, Gemini, and compatible payloads without canonical conversion |
| primitive 不经过转换 | Primitive dispatch boundary forbids canonical parse/emit/transcode | Tests assert returned body equals provider raw body except transport metadata |
| 同一套 budget 计算 | Phase 6 reuses `BudgetTracker`, estimate, reserve, settle, cancellation, and partial usage semantics | Success, provider error, cancellation, and stream/realtime close paths settle exactly once |
| 尽量支持完善 API | Provider API coverage matrix below defines coverage tiers and rollout order | Each provider family has Native / Compatible / Planned coverage status and validation criteria |

## Protocol Architecture

### Canonical Protocol Path

- Entry: existing `Gateway::call` and `Gateway::stream`.
- Request model: `LlmRequest` using OpenAI Responses semantics as the provider-neutral standard.
- Response model: `LlmResponse` and `LlmStreamEvent`.
- Provider reach: existing generation transcoding maps canonical requests to supported provider wire formats.
- Default behavior: canonical path remains default for all current users.
- Budget behavior: unchanged preflight and settlement order from `capability.gateway.execute` and `capability.budget.cost`.

### Primitive Protocol Path

- Entry: new primitive gateway APIs, separate from canonical call/stream APIs.
- Request model: transport-native request with provider, endpoint, wire format, method, path, headers, query, body, stream mode, and metadata.
- Response model: transport-native response preserving provider JSON/text/binary/multipart body plus side-channel usage telemetry.
- Provider reach: provider-native endpoint path, not canonical transcoding.
- Conversion boundary: no `LlmRequest`, `LlmResponse`, `ApiRequest`, `ApiResponse`, parse/emit/transcode, or bridge loss report.
- Budget behavior: shared `BudgetTracker`, shared key pool, shared RPM guard, shared timeout, shared settlement semantics.

## Reference Coverage Targets

| Reference | Planning Role | Required Coverage |
| --- | --- | --- |
| `github.com/openai/openai-go` | OpenAI primitive API shape reference | Responses, Chat Completions, Images, Realtime, Audio Transcriptions, Audio Speech |
| `github.com/anthropics/anthropic-sdk-typescript` | Anthropic primitive API shape reference | Messages, streaming, tool use, token counting, message batches, files, usage/prompt-cache telemetry |
| `github.com/googleapis/go-genai` | Gemini/Vertex primitive API shape reference | GenerateContent, stream GenerateContent, CountTokens, EmbedContent, Live API, files, caches, model operations |
| `github.com/langchain-ai/langchain` | Provider abstraction reference | Provider-specific integrations behind a stable model interface; streaming/tool/structured-output capability detection |
| `github.com/zeroclaw-labs/zeroclaw/.../zeroclaw-providers` | Rust provider runtime reference | Provider trait/factory, OpenAI-compatible support, Anthropic-compatible support, custom providers, fallback/routing patterns |
| `github.com/NousResearch/hermes-agent/.../providers.py` | Runtime provider resolution reference | Provider registry/resolution, credential selection, API mode selection such as Responses, Anthropic Messages, Chat Completions |

## Provider API Coverage Matrix

| Provider Family | Canonical OpenAI Responses Path | Primitive Required APIs | Primitive Expansion APIs | Budget Telemetry Source |
| --- | --- | --- | --- | --- |
| OpenAI | Native standard protocol | Responses, Chat Completions, Images, Realtime, Audio Transcriptions, Audio Speech | Embeddings, Files, Batches, Models, Image edits/variations where transport model supports them | `usage`, response events, audio/image provider fields when available |
| Azure OpenAI | Compatible OpenAI protocol | Responses, Chat Completions, Images, Audio Transcriptions, Audio Speech | Embeddings, Realtime, deployment-specific paths | OpenAI-compatible usage plus Azure deployment metadata |
| Anthropic | Canonical bridge to Messages | Messages, streaming Messages, Count Tokens, Message Batches | Files, Models, beta features behind feature gates | `usage.input_tokens`, `usage.output_tokens`, cache read/write token fields |
| Gemini Developer API | Canonical bridge to GenerateContent | GenerateContent, streamGenerateContent, CountTokens, EmbedContent, Live | Files, Caches, Batches, image generation if available | `usageMetadata` token fields and CountTokens preflight |
| Vertex AI Gemini | Compatible Gemini protocol | GenerateContent, streamGenerateContent, CountTokens, EmbedContent | Rerank, Live, Images, regional/project routing | Vertex/Gemini usage metadata and billable unit fields |
| Bedrock | Planned canonical bridge | Converse, ConverseStream, InvokeModel, InvokeModelWithResponseStream | Provider-specific model endpoints | Bedrock invocation metrics and provider usage blocks |
| OpenAI-Compatible | Compatible OpenAI path | Chat Completions first, Responses when provider supports it | Embeddings, Images, Audio, Rerank | OpenAI-compatible `usage` with conservative fallback |
| Custom HTTP | None by default | User-declared endpoint + auth + usage extractor | User-declared stream/realtime mode | Explicit user-provided estimator or conservative fallback |

## Phase Order

### Phase 1 — Canonical Path Guardrails

- Freeze the existing OpenAI Responses canonical behavior as the default execution path.
- Add regression checks that `Gateway::call`, `Gateway::stream`, provider transcoding, provider registry, and budget settlement remain unchanged.
- Acceptance: all existing canonical tests pass without requiring primitive configuration.

### Phase 2 — Protocol Mode Separation

- Introduce an explicit architecture boundary between `OpenAiResponsesStandard` and `ProviderPrimitive` modes.
- Keep mode selection additive: existing users get canonical mode automatically; primitive mode requires explicit API entry or config.
- Acceptance: no primitive type is required to construct or use the current gateway.

### Phase 3 — Primitive Transport Model

- Define primitive request/response/stream/realtime transport models that preserve raw provider payloads.
- Include JSON, text, binary, multipart, SSE, WebSocket, WebRTC, and provider-native event frames as target transport shapes.
- Acceptance: primitive payload tests prove no canonical schema rewrite occurs.

### Phase 4 — Primitive Provider Registry

- Add a primitive support registry separate from the canonical provider registry.
- Track provider, endpoint, wire format, stream mode, auth scheme, default base URL, and support level.
- Acceptance: unsupported primitive endpoints fail before key acquisition, RPM acquisition, budget reservation, or network dispatch.

### Phase 5 — Provider API Family Implementation Slices

- OpenAI slice: Responses, Chat Completions, Images, Realtime scaffold, Audio Transcriptions, Audio Speech.
- Anthropic slice: Messages, streaming, Count Tokens, Message Batches, Files, prompt-cache usage extraction.
- Gemini slice: GenerateContent, streaming, CountTokens, EmbedContent, Live scaffold, Files, Caches.
- Compatibility slice: OpenAI-compatible Chat Completions and optional Responses.
- Expansion slice: Bedrock, Vertex-specific routing, custom HTTP provider definitions.
- Acceptance: each slice has native request/response preservation tests and usage extraction tests.

### Phase 6 — Unified Budget Accounting

- Reuse the same `BudgetTracker` for canonical and primitive calls.
- Estimate precedence: provider token-count endpoint, provider-specific estimator, existing heuristic, conservative unknown-model rate.
- Settlement precedence: provider token usage, provider billable units with rate table, partial stream/realtime usage, reserved estimate.
- Cancellation: refund before provider response; partially settle after observed usage.
- Acceptance: success, provider error, local rate limit, cancellation, stream EOF, and realtime close each settle exactly once.

### Phase 7 — Primitive Streaming And Realtime

- Add SSE streaming for OpenAI Chat/Responses, Anthropic Messages, and Gemini streamGenerateContent.
- Add realtime scaffolds for OpenAI Realtime and Gemini Live with explicit feature-gated transport support.
- Preserve provider-native event order and content; emit usage as side-channel telemetry, not as payload rewrite.
- Acceptance: stream tests verify frame preservation, usage tracking, cancellation, and EOF settlement.

### Phase 8 — Public API And Documentation

- Re-export primitive types and builders from the crate root.
- Document when to use canonical OpenAI Responses mode versus primitive provider mode.
- Add examples for canonical usage, OpenAI primitive usage, Anthropic primitive usage, Gemini primitive usage, and OpenAI-compatible primitive usage.
- Acceptance: docs and examples make the two protocol forms explicit and do not imply primitive mode is canonical.

## Hard Dependencies

- Existing `Gateway`, `KeyPool`, `BudgetTracker`, `Dispatcher`, and request timeout infrastructure.
- Existing transport body abstractions: `RequestBody`, `ResponseBody`, multipart fields, binary body handling.
- Existing error model for normalized gateway failures.
- Provider docs and SDK references listed in the reference coverage table.

## Design Decisions

| Decision | Chosen Path | Rejected Path | Reason |
| --- | --- | --- | --- |
| Standard protocol | Keep OpenAI Responses canonical | Replace with provider-neutral custom schema | Existing project truth and user requirement both require Responses as standard |
| Primitive semantics | Transport-native, no canonical conversion | Convert primitive into `LlmRequest` internally | Conversion would violate raw provider protocol support |
| Budget | Shared tracker and settlement pipeline | Separate primitive accounting | User requires unified token budget |
| Provider coverage | Coverage matrix with Native/Compatible/Planned levels | Claim all APIs implemented at once | API families differ and need phased validation |
| Usage extraction | Side-channel projection from raw provider payload | Mutate primitive response body into canonical usage | Preserves primitive payload while enabling budget settlement |

## Risks And Mitigations

- Risk: primitive API grows into unbounded SDK parity. Mitigation: require provider API family slices and coverage status before claiming support.
- Risk: primitive response preservation conflicts with usage extraction. Mitigation: usage extraction reads raw fields by reference and stores telemetry separately.
- Risk: budget diverges between canonical and primitive paths. Mitigation: enforce one `BudgetTracker` and one settlement policy with tests for every terminal path.
- Risk: provider-specific auth/path rules leak into canonical mode. Mitigation: primitive endpoint config and registry stay separate from existing canonical `ProviderEndpoint`.
- Risk: realtime and binary APIs have transport complexity. Mitigation: scaffold types first, then enable concrete transports behind capability gates.

## Validation

- Current canonical regression tests remain green.
- Primitive unit tests cover request/response body preservation.
- Primitive registry tests cover unsupported endpoint rejection before budget reservation.
- Budget tests cover reserve/refund/settle-once behavior for canonical and primitive paths.
- Provider family tests cover OpenAI, Anthropic, Gemini, and OpenAI-compatible usage extraction.
- Stream/realtime tests cover event preservation, usage side-channel, cancellation, and EOF/session-close settlement.

## Task Candidates

- Add canonical guardrail tests.
- Add primitive transport model and public API surface.
- Add primitive provider registry and support matrix.
- Add primitive non-streaming dispatcher path.
- Add primitive budget estimator and usage extractor.
- Add OpenAI primitive API family slice.
- Add Anthropic primitive API family slice.
- Add Gemini primitive API family slice.
- Add OpenAI-compatible/custom provider slice.
- Add primitive stream and realtime scaffold.
- Add docs/examples for both protocol modes.
