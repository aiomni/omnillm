# OmniLLM API Reference

This reference summarizes the public OmniLLM surface that most users touch.
Start with the section that matches the user's task, then pull in examples only
if you need runnable code.

## Table Of Contents

- [Runtime Gateway](#runtime-gateway)
- [Canonical Generation Types](#canonical-generation-types)
- [Provider Endpoints And Auth](#provider-endpoints-and-auth)
- [Protocol Conversion](#protocol-conversion)
- [Multi-Endpoint API](#multi-endpoint-api)
- [Embedded Provider Registry](#embedded-provider-registry)
- [Replay Sanitization](#replay-sanitization)
- [Errors And Diagnostics](#errors-and-diagnostics)
- [Live Demo Environment](#live-demo-environment)

## Runtime Gateway

Use the runtime gateway when you want to send live generation requests.

### Core types

- `GatewayBuilder`
- `Gateway`
- `KeyConfig`
- `PoolConfig`
- `ProviderEndpoint`
- `EndpointProtocol`
- `ProviderProtocol`

### Builder flow

```rust
let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
    .add_key(KeyConfig::new("sk-key-1", "prod-1").tpm_limit(90_000).rpm_limit(500))
    .budget_limit_usd(50.0)
    .build()?;
```

Common builder methods:

- `.add_key(key_config)`
- `.add_keys(iterable)`
- `.budget_limit_usd(f64)`
- `.pool_config(PoolConfig)`
- `.request_timeout(Duration)`
- `.build()`

Common `Gateway` methods:

- `gateway.call(request, cancel)`
- `gateway.stream(request, cancel)`
- `gateway.pool_status()`
- `gateway.budget_used_usd()`
- `gateway.budget_remaining_usd()`

`KeyConfig::new(key, label)` stores the raw key plus a human-readable label.
Use `.tpm_limit(...)` and `.rpm_limit(...)` when local quota control matters.
Labels show up in `pool_status()`.

## Canonical Generation Types

OmniLLM normalizes generation around `LlmRequest` and `LlmResponse`.

### `LlmRequest`

Most important fields:

- `model`
- `instructions`
- `input`
- `messages`
- `capabilities`
- `generation`
- `metadata`
- `vendor_extensions`

Guidance:

- Prefer `input` for new code.
- Use `instructions` for top-level system or developer guidance.
- Treat `messages` as a compatibility view when you need chat-shaped input.

Supporting types you will often use:

- `Message`
- `MessageRole`
- `MessagePart`
- `RequestItem`

`Message.parts` is the canonical content model behind chat compatibility input.
When OmniLLM emits OpenAI Chat Completions payloads, plain text stays in
`messages[].content[]` as typed parts rather than collapsing to a bare string.

### `GenerationConfig`

Common controls:

- `max_output_tokens`
- `temperature`
- `top_p`
- `top_k`
- `stop_sequences`
- `presence_penalty`
- `frequency_penalty`
- `seed`

### `CapabilitySet`

Cross-provider capability fields include:

- `tools`
- `structured_output`
- `reasoning`
- `builtin_tools`
- `modalities`
- `safety`
- `cache`

If a target provider cannot represent a capability, conversion may be bridged or
lossy. Surface that explicitly.

### `LlmResponse` and `LlmStreamEvent`

Common response fields and helpers:

- `response.model`
- `response.usage.total()`
- `response.content_text`

Streaming yields `Result<LlmStreamEvent, GatewayError>`. Common events include:

- `ResponseStarted`
- `TextDelta`
- `ToolCallDelta`
- `Usage`
- `Completed`

## Provider Endpoints And Auth

Built-in generation endpoints:

- `ProviderEndpoint::openai_responses()`
- `ProviderEndpoint::openai_chat_completions()`
- `ProviderEndpoint::claude_messages()`
- `ProviderEndpoint::gemini_generate_content()`

Custom or OpenAI-compatible hosts:

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

Use official `EndpointProtocol` variants when `base_url` is a host or prefix
and OmniLLM should derive the standard path. Use `*_compat` variants when
`base_url` is already the full request URL exposed by a wrapper or compatibility
gateway.
`OpenAiChatCompletionsCompat` is the right runtime choice when a wrapper
exposes a full chat-completions URL and insists on strict array-shaped
`content` parts.

`AuthScheme` supports:

- `Bearer`
- `Header { name }`
- `Query { name }`

## Protocol Conversion

Use this layer when you already have raw provider JSON or need wire-level
translation across supported generation protocols.

### Parse and emit helpers

- `parse_request`
- `emit_request`
- `parse_response`
- `emit_response`
- `parse_stream_event`
- `emit_stream_event`
- `parse_error`
- `emit_error`

### Transcoding helpers

- `transcode_request`
- `transcode_response`
- `transcode_stream_event`
- `transcode_error`

Supported generation protocol identifiers in the repo include:

- `ProviderProtocol::OpenAiResponses`
- `ProviderProtocol::OpenAiChatCompletions`
- `ProviderProtocol::ClaudeMessages`
- `ProviderProtocol::GeminiGenerateContent`

These names come from the upstream API families OmniLLM models. Treat
`ProviderProtocol` as the wire-level parse/emit/transcode surface and
`EndpointProtocol` as the runtime endpoint configuration surface.

When you bridge from a richer source protocol to a narrower target protocol,
expect feature loss. For typed API work, use `ConversionReport<T>` to expose
that precisely.

## Multi-Endpoint API

Use the typed multi-endpoint layer for canonical request and response handling
outside runtime generation.

### Canonical request and response wrappers

`ApiRequest` variants:

- `ApiRequest::Responses(LlmRequest)`
- `ApiRequest::Embeddings(EmbeddingRequest)`
- `ApiRequest::ImageGenerations(ImageGenerationRequest)`
- `ApiRequest::AudioTranscriptions(AudioTranscriptionRequest)`
- `ApiRequest::AudioSpeech(AudioSpeechRequest)`
- `ApiRequest::Rerank(RerankRequest)`

`ApiResponse` mirrors the same endpoint families.

### Helper functions

- `emit_api_request`
- `emit_api_response`
- `emit_transport_request`
- `parse_api_request`
- `parse_api_response`
- `parse_transport_response`
- `transcode_api_request`
- `transcode_api_response`

### Wire formats

Common `WireFormat` values in the repo:

- `OpenAiResponses`
- `OpenAiChatCompletions`
- `AnthropicMessages`
- `GeminiGenerateContent`
- `OpenAiEmbeddings`
- `OpenAiImageGenerations`
- `OpenAiAudioTranscriptions`
- `OpenAiAudioSpeech`
- `OpenAiRerank`

### `ConversionReport<T>`

This wrapper carries:

- `value`
- `bridged`
- `lossy`
- `loss_reasons`

Always check `loss_reasons` before claiming a conversion is fully faithful.

### Important limitation

The runtime `Gateway` is for generation requests only. Embeddings, images,
audio, and rerank are available as typed conversion helpers, not as a full
runtime transport client.

## Embedded Provider Registry

Use the registry for metadata, capability discovery, validation, or config UI
logic.

Relevant items:

- `embedded_provider_registry()`
- `ProviderRegistry`
- `ProviderKind`
- `EndpointKind`
- `SupportLevel`
- `EndpointSupport`

This registry is not a runtime dispatcher.

## Replay Sanitization

Use these helpers before storing request and response fixtures:

- `ReplayFixture::sanitized()`
- `sanitize_transport_request`
- `sanitize_transport_response`
- `sanitize_json_value`

By default, the sanitizer redacts:

- authorization headers
- query tokens such as `ak`
- JSON keys such as `api_key`, `token`, and `secret`
- large binary or base64 payloads

## Errors And Diagnostics

Common `GatewayError` variants:

- `NoAvailableKey`
- `BudgetExceeded`
- `RateLimited`
- `Unauthorized`
- `Cancelled`
- `Provider(ProviderError)`
- `Protocol(String)`
- `Http(reqwest::Error)`

Operational guidance:

- `NoAvailableKey`
  All keys are cooling down, dead, or locally saturated.

- `BudgetExceeded`
  OmniLLM rejected the request before dispatch because estimated cost exceeded
  the process-local budget.

- `RateLimited`
  Local RPM admission or an upstream 429 failed the request.

- `Unauthorized`
  The key is marked dead after a 401 or 403.

- `Protocol(String)`
  The payload shape and selected protocol do not match, or a requested feature
  cannot be encoded.

## Live Demo Environment

The repository's live runtime demo uses:

- `OMNILLM_RESPONSES_BASE_URL`
- `OMNILLM_RESPONSES_API_KEY`
- `OMNILLM_RESPONSES_PROTOCOL`
- `OMNILLM_RESPONSES_AUTH_SCHEME`
- `OMNILLM_RESPONSES_AUTH_NAME`
- `OMNILLM_RESPONSES_EXTRA_HEADER_NAME`
- `OMNILLM_RESPONSES_EXTRA_HEADER_VALUE`
- `OMNILLM_RESPONSES_STREAM`
- `OMNILLM_RESPONSES_MAX_OUTPUT_TOKENS`
- `OMNILLM_RESPONSES_VISION_MODEL`
- `OMNILLM_RESPONSES_TOOL_MODEL`
- `OMNILLM_RESPONSES_IMAGE_URL`
- `OMNILLM_RESPONSES_VISION_PROMPT`
- `OMNILLM_RESPONSES_TOOL_PROMPT`

If you see these names in code or docs, use the live runtime configuration
flow instead of hardcoding provider settings.
