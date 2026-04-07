//! Demonstrates the multi-endpoint API layer without making network calls.
//!
//! Run with:
//! ```sh
//! cargo run --example multi_endpoint_demo
//! ```

use omni_gateway::{
    embedded_provider_registry, emit_transport_request, sanitize_transport_request, ApiRequest,
    EmbeddingInput, EmbeddingRequest, ReplayFixture, RequestBody, ResponseBody, TransportRequest,
    TransportResponse, WireFormat,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = embedded_provider_registry();
    let openai = registry
        .provider(omni_gateway::ProviderKind::OpenAi)
        .expect("openai provider should exist");
    println!(
        "OpenAI supports embeddings: {}",
        openai.supports_endpoint(omni_gateway::EndpointKind::Embeddings)
    );

    let raw_chat = json!({
        "model": "gpt-4.1-mini",
        "messages": [{ "role": "user", "content": "Hello!" }],
        "max_tokens": 32
    })
    .to_string();
    let transcoded = omni_gateway::transcode_api_request(
        WireFormat::OpenAiChatCompletions,
        WireFormat::OpenAiResponses,
        &raw_chat,
    )?;
    println!(
        "chat -> responses bridged={} lossy={}",
        transcoded.bridged, transcoded.lossy
    );
    println!("responses payload: {}", transcoded.value);

    let embedding_request = ApiRequest::Embeddings(EmbeddingRequest {
        model: "text-embedding-3-small".into(),
        input: vec![EmbeddingInput::Text {
            text: "hello world".into(),
        }],
        dimensions: Some(256),
        encoding_format: None,
        user: Some("demo-user".into()),
        vendor_extensions: Default::default(),
    });
    let embedding_transport =
        emit_transport_request(WireFormat::OpenAiEmbeddings, &embedding_request)?;
    println!("embedding path: {}", embedding_transport.value.path);
    if let RequestBody::Json { value } = embedding_transport.value.body {
        println!("embedding body: {}", value);
    }

    let fixture = ReplayFixture {
        wire_format: WireFormat::OpenAiResponses,
        request: TransportRequest {
            method: omni_gateway::HttpMethod::Post,
            path: "/responses?ak=secret-ak".into(),
            headers: [("Authorization".into(), "Bearer secret-token".into())]
                .into_iter()
                .collect(),
            accept: None,
            body: RequestBody::Json {
                value: json!({
                    "model": "gpt-4.1-mini",
                    "input": "hello",
                    "api_key": "secret-key"
                }),
            },
        },
        response: TransportResponse {
            status: 200,
            headers: Default::default(),
            content_type: Some("application/json".into()),
            body: ResponseBody::Json {
                value: json!({
                    "id": "resp_demo",
                    "output_text": "Hello back!"
                }),
            },
        },
    };
    println!(
        "sanitized fixture: {}",
        serde_json::to_string_pretty(&fixture.sanitized())?
    );

    let sanitized_request = sanitize_transport_request(&fixture.request);
    println!(
        "sanitized request path: {}, auth={}",
        sanitized_request.path,
        sanitized_request
            .headers
            .get("Authorization")
            .cloned()
            .unwrap_or_default()
    );

    Ok(())
}
