use omnillm::{
    embedded_provider_registry, emit_transport_request, sanitize_transport_request, ApiRequest,
    EmbeddingInput, EmbeddingRequest, HttpMethod, RequestBody, TransportRequest, WireFormat,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_chat = json!({
        "model": "gpt-4.1-mini",
        "messages": [{
            "role": "user",
            "content": [{ "type": "text", "text": "Hello!" }]
        }],
        "max_tokens": 32
    })
    .to_string();

    let transcoded = omnillm::transcode_api_request(
        WireFormat::OpenAiChatCompletions,
        WireFormat::OpenAiResponses,
        &raw_chat,
    )?;
    println!(
        "bridged={} lossy={}",
        transcoded.bridged, transcoded.lossy
    );
    for reason in &transcoded.loss_reasons {
        println!("loss: {reason}");
    }

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
    let emitted = emit_transport_request(WireFormat::OpenAiEmbeddings, &embedding_request)?;
    println!("embedding path: {}", emitted.value.path);
    if let RequestBody::Json { value } = emitted.value.body {
        println!("embedding body: {}", value);
    }

    let registry = embedded_provider_registry();
    println!(
        "OpenAI supports embeddings: {}",
        registry.supports_endpoint(omnillm::ProviderKind::OpenAi, omnillm::EndpointKind::Embeddings)
    );

    let request = TransportRequest {
        method: HttpMethod::Post,
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
    };
    let sanitized = sanitize_transport_request(&request);
    println!("sanitized path: {}", sanitized.path);
    println!(
        "sanitized auth: {}",
        sanitized
            .headers
            .get("Authorization")
            .cloned()
            .unwrap_or_default()
    );

    Ok(())
}
