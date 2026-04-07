use omnillm::{
    embedded_provider_registry, emit_transport_request, sanitize_transport_request, ApiRequest,
    BuiltinTool, CapabilitySet, EmbeddingInput, EmbeddingRequest, GenerationConfig, LlmRequest,
    Message, MessageRole, ReasoningCapability, ReplayFixture, RequestBody, RequestItem,
    ResponseBody, TransportRequest, TransportResponse, WireFormat,
};
use serde_json::json;

#[test]
fn public_api_emits_embedding_transport_request() {
    let request = ApiRequest::Embeddings(EmbeddingRequest {
        model: "text-embedding-3-small".into(),
        input: vec![EmbeddingInput::Text {
            text: "hello".into(),
        }],
        dimensions: Some(256),
        encoding_format: None,
        user: None,
        vendor_extensions: Default::default(),
    });

    let report =
        emit_transport_request(WireFormat::OpenAiEmbeddings, &request).expect("emit transport");

    assert_eq!(report.value.path, "/embeddings");
    let RequestBody::Json { value } = report.value.body else {
        panic!("expected json body");
    };
    assert_eq!(value["dimensions"], 256);
}

#[test]
fn public_registry_exposes_supported_formats() {
    let registry = embedded_provider_registry();

    assert!(registry.supports_wire_format(
        omnillm::ProviderKind::OpenAi,
        WireFormat::OpenAiResponses
    ));
    assert!(!registry.supports_endpoint(
        omnillm::ProviderKind::Bedrock,
        omnillm::EndpointKind::Messages
    ));
}

#[test]
fn public_replay_sanitizer_redacts_authorization_and_query_tokens() {
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("Authorization".into(), "Bearer secret".into());

    let request = TransportRequest {
        method: omnillm::HttpMethod::Post,
        path: "/responses?ak=secret".into(),
        headers,
        accept: None,
        body: RequestBody::Json {
            value: json!({"token":"secret"}),
        },
    };

    let sanitized = sanitize_transport_request(&request);

    assert_eq!(
        sanitized.headers.get("Authorization").map(String::as_str),
        Some("<redacted:Authorization>")
    );
    assert_eq!(sanitized.path, "/responses?ak=<redacted:ak>");
}

#[test]
fn public_generation_transcode_reports_loss_for_downgrade() {
    let request = ApiRequest::Responses(LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: Some("be concise".into()),
        input: vec![RequestItem::from(Message::text(MessageRole::User, "hi"))],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            builtin_tools: vec![BuiltinTool::WebSearch],
            reasoning: Some(ReasoningCapability {
                effort: Some("medium".into()),
                summary: None,
                vendor_extensions: Default::default(),
            }),
            ..Default::default()
        },
        generation: GenerationConfig::default(),
        metadata: [("trace_id".into(), json!("trace-1"))]
            .into_iter()
            .collect(),
        vendor_extensions: Default::default(),
    });

    let raw = omnillm::emit_api_request(WireFormat::OpenAiResponses, &request)
        .expect("emit responses request");
    let downgraded = omnillm::transcode_api_request(
        WireFormat::OpenAiResponses,
        WireFormat::OpenAiChatCompletions,
        &raw.value,
    )
    .expect("transcode request");

    assert!(downgraded.bridged);
    assert!(downgraded.lossy);
    assert!(downgraded
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("builtin tools")));
}

#[test]
fn public_replay_fixture_sanitizes_binary_responses() {
    let fixture = ReplayFixture {
        wire_format: WireFormat::OpenAiAudioSpeech,
        request: TransportRequest {
            method: omnillm::HttpMethod::Post,
            path: "/audio/speech".into(),
            headers: Default::default(),
            accept: Some("audio/mpeg".into()),
            body: RequestBody::Json {
                value: json!({"model":"tts-1","input":"hello"}),
            },
        },
        response: TransportResponse {
            status: 200,
            headers: Default::default(),
            content_type: Some("audio/mpeg".into()),
            body: ResponseBody::Binary {
                data_base64: "ZmFrZQ==".into(),
                media_type: Some("audio/mpeg".into()),
            },
        },
    };

    let sanitized = fixture.sanitized();
    let ResponseBody::Binary { data_base64, .. } = sanitized.response.body else {
        panic!("expected binary response");
    };
    assert_eq!(data_base64, "<redacted:binary_blob>");
}
