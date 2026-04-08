# OmniLLM

An AI-native, production-grade Rust library for provider-neutral LLM access with multi-key load balancing, per-key rate limiting, protocol conversion, circuit breaking, and lock-free cost tracking.

## Documentation

- [Detailed Usage Guide](https://github.com/aiomni/omnillm/blob/main/website/docs/usage.md)
- [Skill Guide](https://github.com/aiomni/omnillm/blob/main/website/docs/skill.md)
- [Architecture Notes](https://github.com/aiomni/omnillm/blob/main/website/docs/architecture.md)
- [Implementation Notes](https://github.com/aiomni/omnillm/blob/main/website/docs/implementation.md)
- [API docs on docs.rs](https://docs.rs/omnillm)
- [OmniLLM Skill Source](./skill)
- [OmniLLM Skill README](./skill/README.md)

## AI-Native Project

OmniLLM ships with a first-party OmniLLM Skill in [`skill/`](./skill). The
skill teaches coding agents how to work with OmniLLM's actual runtime and conversion
surfaces instead of guessing from generic Rust or generic SDK patterns.

The bundled Skill is tuned for repository-native signals such as:

- `GatewayBuilder`, `Gateway`, `KeyConfig`, `PoolConfig`
- `ProviderEndpoint`, `ProviderProtocol`, `LlmRequest`, `LlmStreamEvent`
- `ApiRequest`, `WireFormat`, `emit_transport_request`, `transcode_*`
- `ReplayFixture`, `sanitize_transport_request`, `OMNILLM_RESPONSES_*`
- runtime errors like `NoAvailableKey`, `BudgetExceeded`, and `Protocol(...)`

### Bundled Skill

The repository includes the OmniLLM Skill in [`skill/`](./skill). The
installation guide lives in [`skill/README.md`](./skill/README.md), and the
website version lives in
[`website/docs/skill.md`](./website/docs/skill.md).

### Install The Skill

See [`skill/README.md`](./skill/README.md) for Claude Code, Codex, OpenCode,
and Claude installation instructions.

### Use The Skill

After installing it, ask your agent to:

- integrate `omnillm` into a Rust project
- configure a multi-key runtime gateway
- transcode between provider protocols or typed endpoint formats
- explain replay sanitization and fixture-safe testing
- debug OmniLLM-specific errors and configuration issues

## Repository Docs Site

The documentation site source lives in the GitHub repository:

- [website/docs](https://github.com/aiomni/omnillm/tree/main/website/docs)
- [website/theme](https://github.com/aiomni/omnillm/tree/main/website/theme)
- [skill](https://github.com/aiomni/omnillm/tree/main/skill)
- [GitHub Pages workflow](https://github.com/aiomni/omnillm/blob/main/.github/workflows/gh-pages.yml)

## Features

- Canonical `Responses + Capability Layer` hybrid request/response model
- Additive multi-endpoint API layer with canonical request/response types for generation, embeddings, images, audio, and rerank
- Protocol-aware dispatch for OpenAI Responses, OpenAI Chat Completions, Claude Messages, and Gemini GenerateContent
- Raw JSON and typed transcoders between supported protocols and endpoint families
- Message-level `raw_message` preservation for higher-fidelity round trips
- Embedded provider support registry for OpenAI, Azure OpenAI, Anthropic, Gemini, Vertex AI, Bedrock, and OpenAI-compatible endpoints
- Replay fixture sanitization helpers for safe record/replay style testing
- Multi-key load balancing with per-key rate limiting and circuit breaking
- Lock-free budget tracking with pre-reserve + settle accounting
- Non-streaming `call` and canonical streaming `stream` APIs
- Bundled OmniLLM Skill in `skill/` for AI-native repo guidance

## Canonical Model

Generation stays centered on the existing Response API semantic model:

- `LlmRequest` / `LlmResponse` are still the canonical generation types.
- `ApiRequest` / `ApiResponse` add separate canonical types for embeddings, image generations, audio transcriptions, audio speech, and rerank.
- `ConversionReport<T>` makes bridge semantics explicit with `bridged`, `lossy`, and `loss_reasons`.

This keeps generation normalized around "generate one response" while avoiding capability lock-in to any single wire protocol.

## Endpoint Families

Current typed endpoint coverage:

| Endpoint | Canonical type | Implemented wire formats |
| --- | --- | --- |
| Generation | `LlmRequest` / `LlmResponse` | `open_ai_responses`, `open_ai_chat_completions`, `anthropic_messages`, `gemini_generate_content` |
| Embeddings | `EmbeddingRequest` / `EmbeddingResponse` | `open_ai_embeddings` |
| Image generation | `ImageGenerationRequest` / `ImageGenerationResponse` | `open_ai_image_generations` |
| Audio transcription | `AudioTranscriptionRequest` / `AudioTranscriptionResponse` | `open_ai_audio_transcriptions` |
| Audio speech | `AudioSpeechRequest` / `AudioSpeechResponse` | `open_ai_audio_speech` |
| Rerank | `RerankRequest` / `RerankResponse` | `open_ai_rerank` |

Provider support is exposed through `embedded_provider_registry()`. The registry distinguishes:

- `native`: implemented with provider-native wire format
- `compatible`: OpenAI-compatible or wrapper-style support
- `planned`: listed in the matrix but not yet implemented as a codec/runtime adapter

## Quick Start

```rust
use omnillm::{
    GenerationConfig, GatewayBuilder, KeyConfig, LlmRequest, Message, MessageRole,
    ProviderEndpoint, RequestItem,
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
        .add_key(KeyConfig::new("sk-key-1", "prod-1").tpm_limit(90_000).rpm_limit(500))
        .budget_limit_usd(50.0)
        .build()?;

    let req = LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: Some("Answer concisely".into()),
        input: vec![RequestItem::from(Message::text(MessageRole::User, "Hello!"))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig {
            max_output_tokens: Some(256),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let resp = gateway.call(req, CancellationToken::new()).await?;
    println!("{}", resp.content_text);
    Ok(())
}
```

## Protocol Transcoding

```rust
use omnillm::{transcode_request, ProviderProtocol};

let raw_chat = r#"{
  "model": "gpt-4.1-mini",
  "messages": [{"role": "user", "content": "Hello!"}],
  "max_tokens": 32
}"#;

let raw_responses = transcode_request(
    ProviderProtocol::OpenAiChatCompletions,
    ProviderProtocol::OpenAiResponses,
    raw_chat,
)?;
```

Typed multi-endpoint transcoding keeps bridge metadata:

```rust
use omnillm::{transcode_api_request, WireFormat};

let raw_chat = r#"{
  "model": "gpt-4.1-mini",
  "messages": [{"role": "user", "content": "Hello!"}],
  "max_tokens": 32
}"#;

let report = transcode_api_request(
    WireFormat::OpenAiChatCompletions,
    WireFormat::OpenAiResponses,
    raw_chat,
)?;

assert!(report.bridged);
assert!(!report.lossy);
println!("{}", report.value);
```

If you bridge from the canonical Responses model to a narrower protocol, `loss_reasons` will tell you exactly what was dropped, such as unsupported builtin tools or provider-specific metadata.

## Multi-Endpoint API

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

let transport = emit_transport_request(WireFormat::OpenAiEmbeddings, &request)?;
assert_eq!(transport.value.path, "/embeddings");

if let RequestBody::Json { value } = transport.value.body {
    println!("{}", value);
}
```

Local demo:

```sh
cargo run --example multi_endpoint_demo
```

## Replay Sanitization

`ReplayFixture`, `sanitize_transport_request`, `sanitize_transport_response`, and `sanitize_json_value` are intended for record/replay tests. They redact common secrets by default:

- auth headers
- query tokens such as `ak`
- JSON fields such as `api_key`, `token`, `secret`
- large binary/base64 payload fields

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
        value: json!({ "api_key": "secret", "input": "hello" }),
    },
};

let sanitized = sanitize_transport_request(&request);
assert_eq!(sanitized.path, "/responses?ak=<redacted:ak>");
```

## Live Responses Demo

```sh
cp .env.example .env
cargo run --example responses_live_demo
```

Optional live test:

```sh
cargo test responses_vision_demo -- --ignored --nocapture
cargo test responses_function_tool_demo -- --ignored --nocapture
```

The live demo and live tests read all endpoint configuration from environment variables or a local ignored `.env` file. See `.env.example`.

## Gateway Builder

```rust
use std::time::Duration;
use omnillm::{GatewayBuilder, KeyConfig, PoolConfig, ProviderEndpoint};

let gateway = GatewayBuilder::new(ProviderEndpoint::claude_messages())
    .add_key(KeyConfig::new("sk-key-1", "claude-prod-1"))
    .budget_limit_usd(100.0)
    .pool_config(PoolConfig::default())
    .request_timeout(Duration::from_secs(120))
    .build()
    .expect("at least one key required");
```

## Observability

```rust
for status in gateway.pool_status() {
    println!(
        "Key {:20} available={} inflight={}/{}",
        status.label, status.available, status.tpm_inflight, status.tpm_limit,
    );
}

println!("Budget remaining: ${:.4}", gateway.budget_remaining_usd());
```
