---
name: omnillm
description: |
  Rust provider-neutral LLM gateway and protocol conversion library.
  Use this skill whenever the user mentions `omnillm`, asks to add or configure
  `omnillm` in a Rust project, wants a multi-key LLM gateway, provider-neutral
  generation client, per-key RPM/TPM limiting, circuit breaking, budget tracking,
  protocol conversion, typed endpoint emission/transcoding, or replay fixture
  sanitization. Trigger on code or questions containing `GatewayBuilder`,
  `Gateway`, `KeyConfig`, `PoolConfig`, `ProviderEndpoint`, `EndpointProtocol`,
  `ProviderProtocol`, `LlmRequest`, `LlmResponse`, `LlmStreamEvent`,
  `CapabilitySet`, `ApiRequest`, `ApiResponse`, `WireFormat`,
  `emit_transport_request`, `parse_*`, `emit_*`, `transcode_*`,
  `ReplayFixture`, or env vars like `OMNILLM_RESPONSES_*`.
  Also use this skill for Chinese requests about "多 key 负载均衡", "限流",
  "协议转换", "预算追踪", or errors like `NoAvailableKey`, `BudgetExceeded`,
  `RateLimited`, `Unauthorized`, and `Protocol(...)`.
license: MIT OR Apache-2.0
metadata:
  author: aiomni
  version: 0.1.4
  docs: https://docs.rs/omnillm
  repo: https://github.com/aiomni/omnillm
---

# OmniLLM

OmniLLM is a Rust library for provider-neutral LLM access. It has two major
surfaces:

- a runtime `Gateway` for generation requests with multi-key load balancing,
  per-key RPM/TPM limiting, circuit breaking, budget tracking, and canonical
  streaming
- protocol and typed API conversion helpers for generation, embeddings, image
  generation, audio, and rerank payloads

## Classify The Task First

Before writing code, place the request into exactly one primary lane:

1. Runtime generation
   Use `GatewayBuilder`, `ProviderEndpoint`, `EndpointProtocol`, `KeyConfig`,
   `LlmRequest`, `Gateway::call`, and `Gateway::stream`.

2. Generation protocol parsing or transcoding
   Use `parse_*`, `emit_*`, `transcode_*`, `ProviderProtocol`, and raw JSON
   payloads.

3. Typed multi-endpoint conversion
   Use `ApiRequest`, `ApiResponse`, `WireFormat`, `emit_transport_request`,
   `parse_transport_response`, `transcode_api_*`, and `ConversionReport<T>`.

4. Replay fixture sanitization
   Use `ReplayFixture`, `sanitize_transport_request`,
   `sanitize_transport_response`, and `sanitize_json_value`.

If the user is unsure, infer the lane from code already present in the repo:

- `GatewayBuilder`, `LlmRequest`, `Message`, `RequestItem` usually means runtime
  generation
- `EndpointProtocol` usually means runtime endpoint configuration
- `ProviderProtocol`, `transcode_request`, `emit_request` usually means
  generation protocol work
- `ApiRequest`, `WireFormat`, `emit_transport_request` usually means typed
  multi-endpoint conversion
- `ReplayFixture` or fixture JSON / transport structs usually means
  sanitization or test tooling

## Critical Constraint

Do not blur the runtime and conversion surfaces.

`Gateway` currently transports generation requests only. Embeddings, image
generation, audio transcription, audio speech, and rerank are exposed as
canonical typed conversion helpers, not as a full runtime client surface. If
the user asks for runtime transport of those endpoint families, say that
clearly and guide them toward `emit_transport_request` or `transcode_api_*`
instead.

## Default Workflow

### 1. Add the crate

Use this dependency set for most projects:

```toml
[dependencies]
omnillm = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tokio-util = "0.7"
```

If the user needs native TLS instead of the default `rustls` backend:

```toml
[dependencies]
omnillm = { version = "0.1", default-features = false, features = ["native-tls"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tokio-util = "0.7"
```

### 2. Build requests the OmniLLM way

For new code:

- prefer `LlmRequest.input` over `messages`
- put top-level system or developer guidance in `instructions`
- represent user text with `RequestItem::from(Message::text(...))`
- when targeting OpenAI Chat Completions, build message content with
  `Message.parts` and `MessagePart::*`
- plain-text `MessagePart::Text` is emitted as `messages[].content[]` with
  `{ "type": "text", "text": ... }`, which matters for compat wrappers that
  reject bare string `content`
- keep `capabilities` explicit when the user wants tools, structured output,
  reasoning, builtin tools, or modal output

See `references/examples/basic.rs` for the minimum gateway flow and
`assets/code-template.rs` for a reusable starting point.

### 3. Choose the correct endpoint model

For built-in generation endpoints, prefer:

- `ProviderEndpoint::openai_responses()`
- `ProviderEndpoint::openai_chat_completions()`
- `ProviderEndpoint::claude_messages()`
- `ProviderEndpoint::gemini_generate_content()`

For OpenAI-compatible or custom hosts, construct the endpoint explicitly:

```rust
use omnillm::{AuthScheme, EndpointProtocol, ProviderEndpoint};

let endpoint = ProviderEndpoint::new(
    EndpointProtocol::OpenAiResponsesCompat,
    "https://your-openai-compatible-host/v1/responses",
)
.with_auth(AuthScheme::Header {
    name: "x-api-key".into(),
})
.with_default_header("x-tenant-id", "acme-prod");
```

Use official `EndpointProtocol` variants when OmniLLM should derive the
standard upstream path from a host or prefix. Use `*_compat` variants when the
upstream already exposes the full request URL.
Use `OpenAiChatCompletionsCompat` when a wrapper exposes a full chat endpoint
and still expects the OpenAI Chat wire shape, especially when it insists on
array-shaped `messages[].content[]`.
When a wrapper also requires extra top-level OpenAI request fields that
OmniLLM does not normalize, put them in `LlmRequest.vendor_extensions`.
For OpenAI Responses and OpenAI Chat Completions, OmniLLM emits those
top-level vendor extensions back onto the request body. Use this for wrapper
flags such as `enable_thinking` instead of inventing new canonical fields.

Only override auth if the upstream actually needs it. Keep `ProviderProtocol`
for `parse_*`, `emit_*`, and `transcode_*` work.
Names such as `ClaudeMessages` and `GeminiGenerateContent` belong there because
they mirror upstream wire APIs, not because they are the preferred runtime
configuration surface.

### 4. Make runtime behavior visible

When working on runtime generation flows:

- set labels on `KeyConfig` so pool status is readable
- use `.tpm_limit(...)` and `.rpm_limit(...)` when local admission control
  matters
- use `.budget_limit_usd(...)` when spend needs a process-local guardrail
- use `.request_timeout(...)` for slow models or image/tool flows
- use `gateway.pool_status()`, `gateway.budget_used_usd()`, and
  `gateway.budget_remaining_usd()` for debugging

### 5. Surface bridge semantics instead of hiding them

When using `transcode_*` or `transcode_api_*`:

- inspect `ConversionReport.bridged`
- inspect `ConversionReport.lossy`
- explain `loss_reasons` whenever the target wire format is narrower than the
  source

Do not pretend a downgraded request is lossless if OmniLLM says otherwise.

## Common Patterns

### Runtime gateway

Start here when the user wants a provider-neutral generation client, failover
pool, multi-key routing, or streaming API.

Use:

- `GatewayBuilder`
- `KeyConfig`
- `PoolConfig`
- `ProviderEndpoint`
- `EndpointProtocol`
- `LlmRequest`
- `Gateway::call`
- `Gateway::stream`

Read next:

- `references/examples/basic.rs`
- `references/api-reference.md#runtime-gateway`

### Protocol conversion

Start here when the user already has raw provider JSON or wants to translate
between wire formats.

Use:

- `parse_request`
- `emit_request`
- `parse_response`
- `emit_response`
- `transcode_request`
- `transcode_response`
- `transcode_stream_event`
- `ProviderProtocol`

For OpenAI Chat Completions compat streaming, do not assume one upstream SSE
frame maps to exactly one semantic event. Some wrappers send
`delta.role = "assistant"` and the first `delta.content` in the same frame.
When working on runtime streaming or protocol internals, preserve the first
text delta instead of letting a start event swallow it.

Read next:

- `references/examples/advanced.rs`
- `references/api-reference.md#protocol-conversion`

### Typed multi-endpoint conversion

Start here when the user is working with embeddings, image generation, audio,
rerank, or typed transport payloads.

Use:

- `ApiRequest`
- `ApiResponse`
- `WireFormat`
- `emit_transport_request`
- `parse_transport_response`
- `transcode_api_request`
- `transcode_api_response`

Read next:

- `references/examples/advanced.rs`
- `references/api-reference.md#multi-endpoint-api`

### Replay sanitization

Start here when the user wants safe record/replay fixtures or needs to remove
secrets before storing transport payloads.

Use:

- `ReplayFixture`
- `sanitize_transport_request`
- `sanitize_transport_response`
- `sanitize_json_value`

Read next:

- `references/api-reference.md#replay-sanitization`

## Troubleshooting

**`NoAvailableKey`**
All keys are cooling down, dead, locally saturated on TPM, or blocked by
circuit-breaker state. Inspect `gateway.pool_status()` before changing code
blindly.

**`BudgetExceeded`**
OmniLLM reserves estimated cost before dispatch. Short spikes can make local
usage look temporarily high until settlement finishes.

**`RateLimited`**
The local RPM window rejected the request, or the upstream returned a normalized
429. Check local limits and provider behavior separately.

**`Unauthorized`**
The upstream returned 401 or 403. The affected key is marked dead and will not
be reused.

**`Protocol(...)`**
The payload shape did not match the selected wire format, or the target
protocol cannot encode some requested features. If transcoding, inspect
`loss_reasons`.

**Streaming ended without usage metadata**
This can be normal. The gateway can estimate partial usage and synthesize a
terminal `Completed` event when the upstream does not provide a final canonical
response.

**OpenAI Chat compat stream lost the first text chunk**
Check whether the wrapper coalesced `delta.role` and the first `delta.content`
into the same SSE frame. Runtime streaming should preserve that first text
delta instead of treating the frame as start-only.

## Live Demo Signals

If the repo or user mentions:

- `.env.example`
- `OMNILLM_RESPONSES_BASE_URL`
- `OMNILLM_RESPONSES_API_KEY`
- `OMNILLM_RESPONSES_PROTOCOL`
- `OMNILLM_RESPONSES_AUTH_SCHEME`
- `OMNILLM_RESPONSES_IMAGE_URL`

they are likely following the live runtime demo flow. Parse
`OMNILLM_RESPONSES_PROTOCOL` as `EndpointProtocol`, use `ProviderEndpoint::new(...)`
plus `AuthScheme`, and keep provider configuration in environment variables.

## Output Style

When you answer with code:

- produce compilable Rust that uses exported `omnillm` symbols as named in the
  crate root
- prefer the canonical `input` model
- choose the smallest surface that solves the user's request
- keep provider-specific assumptions explicit
- mention if a requested feature would be bridged, lossy, or conversion-only

For deeper API coverage, read `references/api-reference.md`. For runnable
starting points, read `references/examples/basic.rs`,
`references/examples/advanced.rs`, and `assets/code-template.rs`.
