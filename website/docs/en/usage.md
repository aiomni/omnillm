---
title: Usage Guide
description: Install OmniLLM, configure provider endpoints, send canonical requests, stream results, and operate the runtime in production-shaped flows.
label: runtime guide
release: v0.1.3
updated: Apr 2026
summary: Runtime setup, gateway execution, protocol bridging, budget tracking, replay sanitization, and operational patterns.
---

# Usage Guide

This guide explains how to use OmniLLM as:

- a runtime gateway for generation requests
- a protocol transcoding layer between supported generation APIs
- a typed multi-endpoint conversion layer for embeddings, images, audio, and rerank
- a replay sanitization helper for test fixtures
- a Rust project that ships a first-party OmniLLM Skill

If you want Skill installation details, see [skill.md](./skill.md). If you want
architecture and implementation details, see [architecture.md](./architecture.md)
and [implementation.md](./implementation.md).

## What This Crate Does

OmniLLM has two major surfaces:

1. `Gateway`
   Use this when you want to send generation requests at runtime with:
   - provider-neutral request/response types
   - multi-key load balancing
   - per-key RPM and TPM controls
   - circuit breaking
   - budget tracking
   - canonical streaming events

2. API and protocol conversion helpers
   Use these when you want to:
   - parse raw upstream payloads into canonical types
   - emit canonical types back into provider wire formats
   - transcode between supported protocols
   - inspect bridge and loss metadata
   - sanitize request/response fixtures for tests

Important: the runtime `Gateway` currently handles generation requests only. The embeddings, image, audio, and rerank APIs are exposed as canonical typed conversion helpers, not as a full runtime transport client.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
omnillm = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tokio-util = "0.7"
```

Choose one TLS backend:

- default: `rustls`
- optional: `native-tls`

Examples:

```toml
[dependencies]
omnillm = "0.1"
```

```toml
[dependencies]
omnillm = { version = "0.1", default-features = false, features = ["native-tls"] }
```

## OmniLLM Skill

OmniLLM ships with a first-party agent skill in the repository's
[`skill/` directory](https://github.com/aiomni/omnillm/tree/main/skill). Use
the [Skill Guide](./skill.md) when you want to install it in Claude Code,
Codex, or OpenCode with the Vercel Labs skills workflow.

## Core Concepts

The crate normalizes generation around `LlmRequest` and `LlmResponse`.

- `LlmRequest` is the canonical generation request.
- `LlmResponse` is the canonical generation response.
- `LlmStreamEvent` is the canonical stream event model.
- `CapabilitySet` holds cross-provider features like tools, structured output, reasoning, and builtin tools.
- `EndpointProtocol` identifies a runtime endpoint profile, including `*_compat` modes.
- `ProviderProtocol` identifies a low-level generation wire protocol used by codecs and transcoding.
- `ProviderEndpoint` identifies where and how to send a request.

For multi-endpoint work:

- `ApiRequest` and `ApiResponse` are canonical typed wrappers across endpoint families.
- `WireFormat` identifies a specific upstream wire format.
- `ConversionReport<T>` tells you whether a conversion was bridged and whether data was lost.

## Quick Start

This is the smallest useful runtime setup:

```rust
use omnillm::{
    GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest, Message, MessageRole,
    ProviderEndpoint, RequestItem,
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
        .add_key(KeyConfig::new("sk-key-1", "prod-1"))
        .budget_limit_usd(50.0)
        .build()?;

    let request = LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: Some("Answer concisely.".into()),
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            "Explain Rust ownership in one sentence.",
        ))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig {
            max_output_tokens: Some(128),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let response = gateway.call(request, CancellationToken::new()).await?;
    println!("{}", response.content_text);
    Ok(())
}
```

## Building a Gateway

`GatewayBuilder` controls the runtime client:

```rust
use std::time::Duration;

use omnillm::{GatewayBuilder, KeyConfig, PoolConfig, ProviderEndpoint};

let gateway = GatewayBuilder::new(ProviderEndpoint::claude_messages())
    .add_key(
        KeyConfig::new("sk-key-1", "claude-prod-1")
            .tpm_limit(90_000)
            .rpm_limit(500),
    )
    .add_key(
        KeyConfig::new("sk-key-2", "claude-prod-2")
            .tpm_limit(90_000)
            .rpm_limit(500),
    )
    .budget_limit_usd(100.0)
    .pool_config(PoolConfig::default())
    .request_timeout(Duration::from_secs(120))
    .build()?;
```

### Builder Options

- `add_key` / `add_keys`
  Registers one or more API keys for the same upstream endpoint.

- `budget_limit_usd`
  Sets a process-local budget cap. Requests reserve estimated cost before dispatch and settle to actual cost after completion.

- `pool_config`
  Configures acquire retries and circuit breaker thresholds.

- `request_timeout`
  Sets the HTTP client timeout used by the dispatcher.

### Key Configuration

Each `KeyConfig` contains:

- raw key string
- human-readable label
- `tpm_limit`
- `rpm_limit`

Use labels for observability. Labels are surfaced by `gateway.pool_status()`.

## Choosing a Provider Endpoint

Built-in generation endpoints:

- `ProviderEndpoint::openai_responses()`
- `ProviderEndpoint::openai_chat_completions()`
- `ProviderEndpoint::claude_messages()`
- `ProviderEndpoint::gemini_generate_content()`

You can also construct a custom endpoint:

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

Use a non-`compat` protocol when `base_url` is a host or prefix and OmniLLM should derive the standard path.
Use a `*_compat` protocol when `base_url` is already the full request URL exposed by a wrapper or OpenAI-compatible gateway.
That includes wrappers that expect strict `messages[].content[]` arrays for OpenAI Chat Completions payloads.
`EndpointProtocol` is the runtime configuration surface; names such as `ClaudeMessages` and `GeminiGenerateContent` live on `ProviderProtocol` because they mirror upstream wire APIs used by the parse, emit, and transcode helpers.

### Authentication Modes

`AuthScheme` supports:

- `Bearer`
- `Header { name }`
- `Query { name }`

If you do not set an auth scheme explicitly, `ProviderEndpoint` uses a protocol-specific default.

## Building Requests

### `input` vs `messages`

`LlmRequest` supports both:

- `input`: canonical execution input
- `messages`: compatibility chat-style view

If `input` is non-empty, it is treated as the source of truth. If `input` is empty, `messages` is used.

For new code, prefer `input`.

`Message.parts` is the content model behind the compatibility view. When
OmniLLM emits OpenAI Chat Completions payloads, plain-text chat messages stay
array-shaped: `MessagePart::Text { text: "hi?".into() }` becomes
`content: [{ "type": "text", "text": "hi?" }]`. This is useful for compat
wrappers that reject bare string `content`.

### Provider-Specific Top-Level Fields

Use `LlmRequest.vendor_extensions` for request fields that OmniLLM does not
normalize.

For OpenAI `responses` and `chat_completions`, OmniLLM preserves top-level
request vendor extensions across parse/emit and transport emission. This is
the right place for wrapper-specific flags such as `enable_thinking`.

```rust
use omnillm::{LlmRequest, Message, MessageRole, RequestItem};
use serde_json::json;

let request = LlmRequest {
    model: "openai_qwen3.5-plus".into(),
    input: vec![RequestItem::from(Message::text(
        MessageRole::User,
        "Say hello in Chinese.",
    ))],
    vendor_extensions: [("enable_thinking".into(), json!(false))]
        .into_iter()
        .collect(),
    ..Default::default()
};
```

Keep normalized controls in `generation`, `capabilities`, and `metadata`.
Reach for `vendor_extensions` only when a wrapper needs extra fields that
OmniLLM does not model directly.

### Instructions

`instructions` is the canonical top-level place for system/developer guidance.

If `instructions` is absent, the crate can derive normalized instructions from system/developer messages in the chat-style view.

### Generation Controls

`GenerationConfig` includes:

- `max_output_tokens`
- `temperature`
- `top_p`
- `top_k`
- `stop_sequences`
- `presence_penalty`
- `frequency_penalty`
- `seed`

These are normalized controls. When transcoding to narrower protocols, some fields may be dropped and reported through `ConversionReport.loss_reasons`.

## Capabilities

`CapabilitySet` is the cross-provider capability layer.

### Custom Tools

```rust
use omnillm::{CapabilitySet, ToolDefinition};
use serde_json::json;

let capabilities = CapabilitySet {
    tools: vec![ToolDefinition {
        name: "lookup_weather".into(),
        description: Some("Get current weather for a city".into()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            },
            "required": ["location"]
        }),
        strict: false,
        vendor_extensions: Default::default(),
    }],
    ..Default::default()
};
```

### Structured Output

```rust
use omnillm::{CapabilitySet, StructuredOutputConfig};
use serde_json::json;

let capabilities = CapabilitySet {
    structured_output: Some(StructuredOutputConfig {
        name: Some("summary".into()),
        schema: json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "bullets": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "required": ["title", "bullets"]
        }),
        strict: true,
    }),
    ..Default::default()
};
```

### Reasoning and Builtin Tools

`CapabilitySet` also includes:

- `reasoning`
- `builtin_tools`
- `modalities`
- `safety`
- `cache`

These are canonical abstractions. Support depends on the target protocol. If a target cannot represent part of the capability set, conversion reports mark that as bridged and possibly lossy.

## Non-Streaming Calls

Use `Gateway::call` for one-shot generation:

```rust
let response = gateway.call(request, CancellationToken::new()).await?;

println!("model: {}", response.model);
println!("usage total: {}", response.usage.total());
println!("text: {}", response.content_text);
```

The gateway:

1. estimates tokens and cost
2. acquires a healthy key with enough TPM capacity
3. checks local budget
4. checks the local RPM window
5. dispatches the upstream HTTP request
6. settles cost to actual usage
7. updates key health based on success or failure

## Streaming Calls

Use `Gateway::stream` when you want canonical stream events:

```rust
use futures_util::StreamExt;
use omnillm::LlmStreamEvent;

let mut stream = gateway.stream(request, CancellationToken::new()).await?;

while let Some(event) = stream.next().await {
    match event? {
        LlmStreamEvent::ResponseStarted { model, .. } => {
            println!("started: {}", model);
        }
        LlmStreamEvent::TextDelta { delta } => {
            print!("{delta}");
        }
        LlmStreamEvent::ToolCallDelta { call_id, name, delta } => {
            println!("tool call {call_id} {name}: {delta}");
        }
        LlmStreamEvent::Usage { usage } => {
            println!("usage so far: {}", usage.total());
        }
        LlmStreamEvent::Completed { response } => {
            println!("\nfinal text: {}", response.content_text);
        }
        other => {
            println!("event: {:?}", other);
        }
    }
}
```

### Stream Semantics

- The stream yields `Result<LlmStreamEvent, GatewayError>`.
- Some upstreams send a terminal `Completed` event; others end with `[DONE]` or protocol-specific stop markers.
- The gateway synthesizes a terminal `Completed` event when needed so callers still get a final canonical response.
- If a stream ends or fails before usage metadata is available, the gateway falls back to internal usage estimation to settle budget instead of refunding the whole reservation.

### Cancellation

Use `CancellationToken` to stop an in-flight request:

```rust
let cancel = CancellationToken::new();
let child = cancel.clone();

tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    child.cancel();
});

let result = gateway.call(request, cancel).await;
```

Cancellation becomes `GatewayError::Cancelled`.

## Budget Tracking

Budget tracking is process-local and lock-free.

Key points:

- requests reserve estimated cost before dispatch
- final cost is settled against actual usage
- successful requests can settle up or down
- failed or truncated streams do not automatically refund everything; the gateway uses known or estimated partial usage when possible

Observability methods:

- `gateway.budget_used_usd()`
- `gateway.budget_remaining_usd()`

Use `BudgetTracker` directly if you need the low-level primitive outside of `Gateway`.

## Key Pooling, Rate Limits, and Circuit Breaking

Each key is tracked independently.

### What the Pool Enforces

- TPM reservation using atomic in-flight counters
- RPM admission via a sliding window
- randomized selection to reduce contention
- cooldown on provider rate-limit responses
- permanent death on unauthorized responses
- circuit breaking on repeated provider failures

### Observability

```rust
for status in gateway.pool_status() {
    println!(
        "{} available={} inflight={}/{} failures={}",
        status.label,
        status.available,
        status.tpm_inflight,
        status.tpm_limit,
        status.consecutive_failures,
    );
}
```

`KeyStatus` includes:

- `label`
- `available`
- `tpm_inflight`
- `tpm_limit`
- `cool_down_until`
- `failure_cool_down_until`
- `consecutive_failures`

The cooldown fields are Unix epoch milliseconds.

## Error Handling

Public runtime errors are normalized as `GatewayError`:

- `NoAvailableKey`
- `BudgetExceeded`
- `RateLimited`
- `Unauthorized`
- `Cancelled`
- `Provider(ProviderError)`
- `Protocol(String)`
- `Http(reqwest::Error)`

General guidance:

- `NoAvailableKey`
  No currently healthy key had enough local capacity.

- `BudgetExceeded`
  Your configured budget cap rejected the request before dispatch.

- `RateLimited`
  The local RPM window denied the request, or an upstream 429 was normalized.

- `Unauthorized`
  The upstream returned 401/403. The affected key is marked dead.

- `Provider`
  The transport completed but the provider failed, or the network was normalized as a provider-side failure.

- `Protocol`
  The crate could not parse or emit the expected protocol payload.

## Protocol Parsing and Emission

Use these helpers when you want to work directly with supported generation protocols:

- `parse_request`
- `emit_request`
- `parse_response`
- `emit_response`
- `parse_stream_event`
- `emit_stream_event`
- `transcode_request`
- `transcode_response`
- `transcode_stream_event`
- `transcode_error`

Example:

```rust
use omnillm::{transcode_request, ProviderProtocol};

let raw_chat = r#"{
  "model": "gpt-4.1-mini",
  "messages": [{
    "role": "user",
    "content": [{ "type": "text", "text": "Hello!" }]
  }],
  "max_tokens": 32
}"#;

let raw_responses = transcode_request(
    ProviderProtocol::OpenAiChatCompletions,
    ProviderProtocol::OpenAiResponses,
    raw_chat,
)?;
```

## Multi-Endpoint API Layer

The multi-endpoint API layer is typed and canonical. It is useful when you want to build converters or request emitters for non-generation endpoint families.

### Supported Canonical Endpoint Families

- generation: `ApiRequest::Responses`
- embeddings: `ApiRequest::Embeddings`
- image generation: `ApiRequest::ImageGenerations`
- audio transcription: `ApiRequest::AudioTranscriptions`
- audio speech: `ApiRequest::AudioSpeech`
- rerank: `ApiRequest::Rerank`

### Emitting a Transport Request

```rust
use omnillm::{
    emit_transport_request, ApiRequest, EmbeddingInput, EmbeddingRequest, RequestBody, WireFormat,
};

let request = ApiRequest::Embeddings(EmbeddingRequest {
    model: "text-embedding-3-small".into(),
    input: vec![EmbeddingInput::Text { text: "hello".into() }],
    dimensions: Some(256),
    encoding_format: None,
    user: None,
    vendor_extensions: Default::default(),
});

let report = emit_transport_request(WireFormat::OpenAiEmbeddings, &request)?;
assert_eq!(report.value.path, "/embeddings");

if let RequestBody::Json { value } = report.value.body {
    println!("{}", value);
}
```

### Bridge and Loss Reporting

`ConversionReport<T>` tells you:

- `bridged`
  The target wire format did not natively match the canonical endpoint model.

- `lossy`
  One or more fields could not be represented.

- `loss_reasons`
  A specific explanation of what was dropped or degraded.

Example:

```rust
use omnillm::{transcode_api_request, WireFormat};

let raw = r#"{
  "model": "gpt-4.1-mini",
  "messages": [{
    "role": "user",
    "content": [{ "type": "text", "text": "Hello!" }]
  }],
  "max_tokens": 32
}"#;

let report = transcode_api_request(
    WireFormat::OpenAiChatCompletions,
    WireFormat::OpenAiResponses,
    raw,
)?;

println!("bridged={} lossy={}", report.bridged, report.lossy);
for reason in &report.loss_reasons {
    println!("loss: {}", reason);
}
```

## Embedded Provider Registry

Use the embedded registry to inspect which providers and endpoint families are currently modeled:

```rust
use omnillm::{embedded_provider_registry, EndpointKind, ProviderKind, WireFormat};

let registry = embedded_provider_registry();

assert!(registry.supports_endpoint(ProviderKind::OpenAi, EndpointKind::Embeddings));
assert!(registry.supports_wire_format(
    ProviderKind::OpenAi,
    WireFormat::OpenAiResponses,
));
```

This registry is metadata, not a runtime dispatcher. It helps with capability discovery, configuration UIs, and validation.

## Replay Sanitization

For record/replay style testing, use:

- `ReplayFixture`
- `sanitize_transport_request`
- `sanitize_transport_response`
- `sanitize_json_value`

These helpers redact common secrets such as:

- authorization headers
- query tokens
- JSON key-like secrets
- large binary/base64 blobs

Example:

```rust
use omnillm::{sanitize_transport_request, HttpMethod, RequestBody, TransportRequest};
use serde_json::json;

let request = TransportRequest {
    method: HttpMethod::Post,
    path: "/responses?ak=secret".into(),
    headers: [("Authorization".into(), "Bearer secret".into())]
        .into_iter()
        .collect(),
    accept: None,
    body: RequestBody::Json {
        value: json!({
            "api_key": "secret",
            "input": "hello"
        }),
    },
};

let sanitized = sanitize_transport_request(&request);
assert_eq!(sanitized.path, "/responses?ak=<redacted:ak>");
```

## Examples Included in This Repository

Run these from the repository root:

```sh
cargo run --example basic_usage
cargo run --example multi_endpoint_demo
cargo run --example responses_live_demo
```

What each one shows:

- `basic_usage`
  Concurrent runtime generation calls with budget and pool status printing.

- `multi_endpoint_demo`
  Typed request emission, transcoding, provider registry lookup, and replay sanitization without making network calls.

- `responses_live_demo`
  A live image-capable runtime request configured entirely from environment variables.

## Live Demo and Live Tests

The repository includes `.env.example` for the live runtime demo and ignored live tests.

Typical flow:

```sh
cp .env.example .env
cargo run --example responses_live_demo
```

Optional ignored tests:

```sh
cargo test responses_vision_demo -- --ignored --nocapture
cargo test responses_function_tool_demo -- --ignored --nocapture
```

## Practical Patterns

### 1. OpenAI-compatible Runtime Gateway

Use `ProviderEndpoint::new(...)` with `EndpointProtocol` for runtime configuration.
Use official variants when OmniLLM should derive standard upstream paths, and `*_compat` variants when you need to hit a wrapper-specific full URL while reusing the same wire protocol.

### 2. Conversion-Only Service

If you are writing a proxy, SDK adapter, or test harness, you may never need `Gateway`. Use the `emit_*`, `parse_*`, and `transcode_*` helpers directly.

### 3. Safe Fixture Capture

If you store request/response fixtures in a repository, sanitize them before writing to disk.

## Troubleshooting

### I get `NoAvailableKey`

Possible causes:

- all keys are cooling down
- all keys are dead
- all keys are locally saturated on TPM
- local circuit breaker has opened on all keys

Inspect `gateway.pool_status()`.

### I get `BudgetExceeded` earlier than expected

Remember that the gateway reserves estimated cost before dispatch. The reservation settles later. During spikes, current usage can temporarily look higher until requests settle.

### I get `Protocol(...)`

This usually means one of:

- the upstream payload shape changed
- the selected protocol does not match the upstream
- a feature was requested that the target protocol cannot encode

If you are transcoding, inspect `loss_reasons`.

### Stream ended without provider usage metadata

This is expected for some upstream streaming shapes. The gateway falls back to partial usage estimation for budget settlement and can synthesize a terminal completed response when necessary.

## API Surface Reference

The most commonly used items are:

- runtime generation:
  `Gateway`, `GatewayBuilder`, `KeyConfig`, `PoolConfig`, `ProviderEndpoint`, `EndpointProtocol`

- canonical generation types:
  `LlmRequest`, `LlmResponse`, `LlmStreamEvent`, `Message`, `RequestItem`, `CapabilitySet`

- conversion helpers:
  `parse_request`, `emit_request`, `parse_response`, `emit_response`, `transcode_request`, `transcode_response`

- multi-endpoint API:
  `ApiRequest`, `ApiResponse`, `WireFormat`, `ConversionReport`, `emit_transport_request`, `parse_transport_response`

- replay sanitization:
  `ReplayFixture`, `sanitize_transport_request`, `sanitize_transport_response`, `sanitize_json_value`

## Recommended Reading Order

If you are new to the crate:

1. read the main `README.md`
2. run `cargo run --example basic_usage`
3. read this usage guide
4. read [architecture.md](./architecture.md) if you need design context
5. read [implementation.md](./implementation.md) if you need internals
