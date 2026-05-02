# OmniLLM Agent Instructions

## Project Positioning

- OmniLLM is an LLM gateway, budget tracker, and protocol bridge. It is not a full provider admin SDK.
- Keep OpenAI Responses as the canonical standard protocol. Existing `Gateway::call` and `Gateway::stream` behavior must remain backward compatible.
- Provider primitive protocol support is additive and explicit. Primitive calls preserve provider-native payloads and must not route through `LlmRequest`, `LlmResponse`, `ApiRequest`, or `ApiResponse` conversion.
- Canonical and primitive paths must share the same key pool, RPM protection, timeout model, and budget settlement rules.

## Primitive Provider Scope

- Prefer adding provider-native model invocation, token counting, embeddings, audio/image generation or transcription, minimal file upload/read, model metadata, async batch lifecycle, SSE streams, realtime sessions, and binary media streaming.
- Do not add provider admin, billing, audit, webhook, organization/project/user/key management, fine-tuning, evals, graders, tunings, managed-agent platforms, hosted RAG/vector-store administration, or SDK convenience helpers unless a current Spec explicitly brings them into scope.
- Treat realtime, WebSocket, WebRTC, binary chunk streaming, and async job APIs as separate transport/lifecycle slices with their own budget and cancellation acceptance tests.
- Use registry support levels conservatively. Do not claim full provider SDK parity unless tests cover the transport, request path, response preservation, error behavior, and budget settlement.

## Spec And Plan Workflow

- `omni-coding/specs/current/` is the only authoritative Spec state. Keep behavior truth there before implementing changes.
- `omni-coding/plans/current/` contains implementation strategy only. Do not hide new hard constraints in a Plan.
- Archive/log files are history and should not be used as current truth unless the user asks for historical analysis.
- When expanding primitive provider coverage, update the primitive scope Spec before adding registry entries or runtime code.

## Development Commands

- Format: `cargo fmt`
- Format check: `cargo fmt --check`
- Targeted primitive tests: `cargo test primitive --tests`
- Public API tests: `cargo test --test api_surface`
- Full tests: `cargo test`
- Example compile check: `cargo check --examples`

## Coding Rules

- Keep changes minimal and consistent with existing Rust style.
- Fix root causes instead of adding surface patches.
- Do not add a second budget subsystem.
- Do not mutate primitive request or response bodies when extracting telemetry.
- Do not commit or create branches unless the user explicitly asks.
