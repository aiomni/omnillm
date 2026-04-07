# omni-gateway

A production-grade Rust library for provider-neutral LLM access with multi-key load balancing, per-key rate limiting, protocol conversion, circuit breaking, and lock-free cost tracking.

## Features

- Canonical `Responses + Capability Layer` hybrid request/response model
- Protocol-aware dispatch for OpenAI Responses, OpenAI Chat Completions, Claude Messages, and Gemini GenerateContent
- Raw JSON request/response/error/stream transcoders between supported protocols
- Message-level `raw_message` preservation for higher-fidelity round trips
- Multi-key load balancing with per-key rate limiting and circuit breaking
- Lock-free budget tracking with pre-reserve + settle accounting
- Non-streaming `call` and canonical streaming `stream` APIs

## Quick Start

```rust
use omni_gateway::{
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
use omni_gateway::{transcode_request, ProviderProtocol};

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

## Gateway Builder

```rust
use std::time::Duration;
use omni_gateway::{GatewayBuilder, KeyConfig, PoolConfig, ProviderEndpoint};

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
