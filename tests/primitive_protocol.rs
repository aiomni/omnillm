use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use omnillm::{
    embedded_primitive_provider_registry, GatewayBuilder, GatewayError, KeyConfig, MultipartField,
    MultipartValue, PrimitiveAsyncJobOperation, PrimitiveAsyncJobRequest, PrimitiveAsyncJobStatus,
    PrimitiveBudgetClass, PrimitiveEndpointKind, PrimitiveProviderEndpoint, PrimitiveProviderKind,
    PrimitiveRequest, PrimitiveStreamEvent, PrimitiveStreamMode, PrimitiveSupportTier,
    ProviderEndpoint, ProviderPrimitiveWireFormat, RequestBody, ResponseBody,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request as WsRequest, Response as WsResponse},
        Message as WsMessage,
    },
};
use tokio_util::sync::CancellationToken;

#[test]
fn primitive_registry_gates_endpoint_wire_and_stream_mode() {
    let registry = embedded_primitive_provider_registry();

    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::OpenAi,
        ProviderPrimitiveWireFormat::OpenAiResponses,
        PrimitiveStreamMode::None,
    ));
    assert!(!registry.supports_request(&PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Images,
        ProviderPrimitiveWireFormat::OpenAiChatCompletions,
        "gpt-4o",
        json!({"model":"gpt-4o"}),
    )));
    assert!(!registry.supports_endpoint(
        PrimitiveProviderKind::Bedrock,
        PrimitiveEndpointKind::Messages,
    ));
}

#[test]
fn primitive_registry_exposes_scope_tiers_and_budget_classes() {
    let registry = embedded_primitive_provider_registry();
    let openai = registry
        .provider(PrimitiveProviderKind::OpenAi)
        .expect("openai descriptor");
    let responses = openai
        .endpoints
        .iter()
        .find(|support| support.endpoint == PrimitiveEndpointKind::Responses)
        .expect("responses support");
    assert_eq!(responses.scope_tier, PrimitiveSupportTier::P0KeepAndHarden);
    assert_eq!(responses.budget_class, PrimitiveBudgetClass::TokenMetered);

    let realtime = openai
        .endpoints
        .iter()
        .find(|support| support.endpoint == PrimitiveEndpointKind::Realtime)
        .expect("realtime support");
    assert_eq!(
        realtime.scope_tier,
        PrimitiveSupportTier::P3TransportExpansion
    );
    assert_eq!(realtime.budget_class, PrimitiveBudgetClass::TokenMetered);

    let audio = openai
        .endpoints
        .iter()
        .find(|support| support.endpoint == PrimitiveEndpointKind::AudioSpeech)
        .expect("audio support");
    assert_eq!(
        audio.budget_class,
        PrimitiveBudgetClass::BillableUnitMetered
    );

    assert!(registry
        .providers
        .iter()
        .flat_map(|provider| &provider.endpoints)
        .all(|support| support.scope_tier != PrimitiveSupportTier::Deferred));
}

#[test]
fn primitive_public_model_roundtrips_without_canonical_shape() {
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::Anthropic,
        PrimitiveEndpointKind::Messages,
        ProviderPrimitiveWireFormat::AnthropicMessages,
        "claude-3-5-sonnet-20241022",
        json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [{"role":"user","content":"hello"}],
            "max_tokens": 32
        }),
    )
    .with_header("anthropic-beta", "tools-2024-05-16");

    let encoded = serde_json::to_string(&request).expect("serialize primitive request");
    assert!(encoded.contains("anthropic_messages"));
    assert!(!encoded.contains("LlmRequest"));

    let decoded: PrimitiveRequest =
        serde_json::from_str(&encoded).expect("decode primitive request");
    assert_eq!(decoded.provider, PrimitiveProviderKind::Anthropic);
    assert_eq!(decoded.endpoint, PrimitiveEndpointKind::Messages);
}

#[tokio::test]
async fn primitive_openai_p1_http_gaps_resolve_paths_and_budget_classes() {
    let registry = embedded_primitive_provider_registry();
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::OpenAi,
        ProviderPrimitiveWireFormat::OpenAiFiles,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::OpenAi,
        ProviderPrimitiveWireFormat::OpenAiUploads,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::OpenAi,
        ProviderPrimitiveWireFormat::OpenAiModels,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::OpenAi,
        ProviderPrimitiveWireFormat::OpenAiImageEdits,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::OpenAi,
        ProviderPrimitiveWireFormat::OpenAiAudioTranslations,
        PrimitiveStreamMode::None,
    ));

    let models = PrimitiveRequest::get(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Models,
        ProviderPrimitiveWireFormat::OpenAiModels,
        Option::<String>::None,
    );
    assert_eq!(
        models.budget_class(),
        PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost
    );
    let (response, raw_request, used_usd) =
        call_and_capture_openai(models, json!({"data":[]})).await;
    assert_eq!(response.status, 200);
    assert!(raw_request.starts_with("GET /models HTTP/1.1"));
    assert_eq!(used_usd, 0.0);

    let file_upload = PrimitiveRequest {
        provider: PrimitiveProviderKind::OpenAi,
        endpoint: PrimitiveEndpointKind::Files,
        wire_format: ProviderPrimitiveWireFormat::OpenAiFiles,
        model: None,
        method: omnillm::HttpMethod::Post,
        path: None,
        query: Default::default(),
        headers: Default::default(),
        accept: None,
        body: RequestBody::Multipart {
            fields: vec![
                MultipartField {
                    name: "purpose".into(),
                    value: MultipartValue::Text {
                        value: "batch".into(),
                    },
                },
                MultipartField {
                    name: "file".into(),
                    value: MultipartValue::File {
                        filename: "input.jsonl".into(),
                        data_base64: "e30K".into(),
                        media_type: Some("application/jsonl".into()),
                    },
                },
            ],
        },
        stream: PrimitiveStreamMode::None,
        metadata: Default::default(),
    };
    assert_eq!(
        file_upload.budget_class(),
        PrimitiveBudgetClass::UploadOrStorage
    );
    let (_, raw_request, used_usd) =
        call_and_capture_openai(file_upload, json!({"id":"file_1"})).await;
    assert!(raw_request.starts_with("POST /files HTTP/1.1"));
    assert!(raw_request.contains("name=\"purpose\""));
    assert_eq!(used_usd, 0.0);

    let image_edit = PrimitiveRequest {
        provider: PrimitiveProviderKind::OpenAi,
        endpoint: PrimitiveEndpointKind::Images,
        wire_format: ProviderPrimitiveWireFormat::OpenAiImageEdits,
        model: Some("gpt-image-1".into()),
        method: omnillm::HttpMethod::Post,
        path: None,
        query: Default::default(),
        headers: Default::default(),
        accept: None,
        body: RequestBody::Multipart { fields: Vec::new() },
        stream: PrimitiveStreamMode::None,
        metadata: Default::default(),
    };
    let (_, raw_request, used_usd) = call_and_capture_openai(image_edit, json!({"data":[]})).await;
    assert!(raw_request.starts_with("POST /images/edits HTTP/1.1"));
    assert!(used_usd > 0.0);

    let translation = PrimitiveRequest {
        provider: PrimitiveProviderKind::OpenAi,
        endpoint: PrimitiveEndpointKind::AudioTranslations,
        wire_format: ProviderPrimitiveWireFormat::OpenAiAudioTranslations,
        model: Some("whisper-1".into()),
        method: omnillm::HttpMethod::Post,
        path: None,
        query: Default::default(),
        headers: Default::default(),
        accept: None,
        body: RequestBody::Multipart { fields: Vec::new() },
        stream: PrimitiveStreamMode::None,
        metadata: Default::default(),
    };
    let (_, raw_request, _) = call_and_capture_openai(translation, json!({"text":"hello"})).await;
    assert!(raw_request.starts_with("POST /audio/translations HTTP/1.1"));
}

#[tokio::test]
async fn primitive_call_preserves_openai_payload_and_settles_actual_usage() {
    let response = json!({
        "id": "resp_primitive",
        "output_text": "ok",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5,
            "total_tokens": 15,
            "input_tokens_details": {"cached_tokens": 4}
        }
    });
    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        response.to_string().into_bytes(),
    )
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));

    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Responses,
        ProviderPrimitiveWireFormat::OpenAiResponses,
        "gpt-5.4",
        json!({"model":"gpt-5.4","input":"hello","max_output_tokens":5}),
    )
    .with_header("x-trace", "trace-1")
    .with_query("request_id", "abc");

    let response = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect("primitive call succeeds");

    let ResponseBody::Json { value } = response.body else {
        panic!("expected json primitive body");
    };
    assert_eq!(value["id"], "resp_primitive");
    let usage = response.usage.expect("usage telemetry");
    let token_usage = usage.token_usage.expect("token usage");
    assert_eq!(token_usage.prompt_tokens, 10);
    assert_eq!(token_usage.completion_tokens, 5);
    assert_eq!(
        token_usage
            .prompt_cache
            .and_then(|usage| usage.cached_input_tokens),
        Some(4)
    );
    assert!((gateway.budget_used_usd() - 0.000091).abs() < 1e-12);

    let raw_request = server.await.expect("server joins").expect("server ok");
    let lower = raw_request.to_ascii_lowercase();
    assert!(raw_request.starts_with("POST /responses?request_id=abc HTTP/1.1"));
    assert!(lower.contains("authorization: bearer sk-test"));
    assert!(lower.contains("x-trace: trace-1"));
    assert!(raw_request.contains("\"input\":\"hello\""));
}

#[tokio::test]
async fn unsupported_primitive_request_fails_before_budget_or_network() {
    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        json!({"unexpected": true}).to_string().into_bytes(),
    )
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));

    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Images,
        ProviderPrimitiveWireFormat::OpenAiChatCompletions,
        "gpt-4o",
        json!({"model":"gpt-4o"}),
    );

    let err = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect_err("unsupported endpoint should fail");

    assert!(matches!(err, GatewayError::Protocol(_)));
    assert_eq!(gateway.budget_used_usd(), 0.0);
    server.abort();
}

#[tokio::test]
async fn primitive_anthropic_p1_models_and_files_keep_headers_and_zero_budget() {
    let registry = embedded_primitive_provider_registry();
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::Anthropic,
        ProviderPrimitiveWireFormat::AnthropicModels,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::Anthropic,
        ProviderPrimitiveWireFormat::AnthropicFiles,
        PrimitiveStreamMode::None,
    ));

    let models = PrimitiveRequest::get(
        PrimitiveProviderKind::Anthropic,
        PrimitiveEndpointKind::Models,
        ProviderPrimitiveWireFormat::AnthropicModels,
        Option::<String>::None,
    );
    assert_eq!(
        models.budget_class(),
        PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost
    );
    let (response, raw_request, used_usd) =
        call_and_capture_anthropic(models, json!({"data":[]})).await;
    assert_eq!(response.status, 200);
    assert!(raw_request.starts_with("GET /models HTTP/1.1"));
    let lower = raw_request.to_ascii_lowercase();
    assert!(lower.contains("x-api-key: sk-test"));
    assert!(lower.contains("anthropic-version: 2023-06-01"));
    assert_eq!(used_usd, 0.0);

    let file_get = PrimitiveRequest::get(
        PrimitiveProviderKind::Anthropic,
        PrimitiveEndpointKind::Files,
        ProviderPrimitiveWireFormat::AnthropicFiles,
        Option::<String>::None,
    )
    .with_path("/files/file_1");
    assert_eq!(
        file_get.budget_class(),
        PrimitiveBudgetClass::UploadOrStorage
    );
    let (_, raw_request, used_usd) =
        call_and_capture_anthropic(file_get, json!({"id":"file_1"})).await;
    assert!(raw_request.starts_with("GET /files/file_1 HTTP/1.1"));
    assert_eq!(used_usd, 0.0);
}

#[tokio::test]
async fn primitive_async_job_batch_lifecycle_supports_openai_anthropic_and_gemini() {
    let registry = embedded_primitive_provider_registry();
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::OpenAi,
        ProviderPrimitiveWireFormat::OpenAiBatches,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::Anthropic,
        ProviderPrimitiveWireFormat::AnthropicMessageBatches,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::Gemini,
        ProviderPrimitiveWireFormat::GeminiBatches,
        PrimitiveStreamMode::None,
    ));

    let openai_create = PrimitiveAsyncJobRequest::new(
        PrimitiveAsyncJobOperation::Create,
        PrimitiveRequest::json(
            PrimitiveProviderKind::OpenAi,
            PrimitiveEndpointKind::Batches,
            ProviderPrimitiveWireFormat::OpenAiBatches,
            "batch",
            json!({"input_file_id":"file_1","endpoint":"/v1/responses"}),
        ),
    );
    let (response, raw_request, used_usd) = call_async_job_capture(
        PrimitiveProviderEndpoint::openai(),
        openai_create,
        json!({"id":"batch_1","status":"validating"}),
    )
    .await;
    assert_eq!(response.job_id.as_deref(), Some("batch_1"));
    assert_eq!(response.status, PrimitiveAsyncJobStatus::Pending);
    assert!(raw_request.starts_with("POST /batches HTTP/1.1"));
    assert_eq!(used_usd, 0.0);

    let anthropic_get = PrimitiveAsyncJobRequest::new(
        PrimitiveAsyncJobOperation::Get,
        PrimitiveRequest::get(
            PrimitiveProviderKind::Anthropic,
            PrimitiveEndpointKind::Batches,
            ProviderPrimitiveWireFormat::AnthropicMessageBatches,
            Option::<String>::None,
        )
        .with_path("/messages/batches/msgbatch_1"),
    );
    let (response, raw_request, used_usd) = call_async_job_capture(
        PrimitiveProviderEndpoint::anthropic(),
        anthropic_get,
        json!({"id":"msgbatch_1","status":"processing"}),
    )
    .await;
    assert_eq!(response.status, PrimitiveAsyncJobStatus::Running);
    assert!(raw_request.starts_with("GET /messages/batches/msgbatch_1 HTTP/1.1"));
    assert_eq!(used_usd, 0.0);

    let gemini_cancel = PrimitiveAsyncJobRequest::new(
        PrimitiveAsyncJobOperation::Cancel,
        PrimitiveRequest::json(
            PrimitiveProviderKind::Gemini,
            PrimitiveEndpointKind::Batches,
            ProviderPrimitiveWireFormat::GeminiBatches,
            "batch",
            json!({}),
        )
        .with_path("/batches/batch_1:cancel"),
    );
    let (_, raw_request, used_usd) = call_async_job_capture(
        PrimitiveProviderEndpoint::gemini(),
        gemini_cancel,
        json!({"name":"batch_1","status":"cancelled"}),
    )
    .await;
    assert!(raw_request.starts_with("POST /batches/batch_1:cancel HTTP/1.1"));
    assert_eq!(used_usd, 0.0);

    let openai_results = PrimitiveAsyncJobRequest::new(
        PrimitiveAsyncJobOperation::Results,
        PrimitiveRequest::get(
            PrimitiveProviderKind::OpenAi,
            PrimitiveEndpointKind::Batches,
            ProviderPrimitiveWireFormat::OpenAiBatches,
            Some("gpt-4o"),
        )
        .with_path("/batches/batch_1/results"),
    );
    let (response, raw_request, used_usd) = call_async_job_capture(
        PrimitiveProviderEndpoint::openai(),
        openai_results,
        json!({"id":"batch_1","status":"completed","usage":{"prompt_tokens":4,"completion_tokens":1,"total_tokens":5}}),
    )
    .await;
    assert_eq!(response.status, PrimitiveAsyncJobStatus::Succeeded);
    assert!(raw_request.starts_with("GET /batches/batch_1/results HTTP/1.1"));
    assert!(used_usd > 0.0);
}

#[tokio::test]
async fn primitive_gemini_p1_models_operations_files_and_caches_are_zero_or_storage_budget() {
    let registry = embedded_primitive_provider_registry();
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::Gemini,
        ProviderPrimitiveWireFormat::GeminiModels,
        PrimitiveStreamMode::None,
    ));
    assert!(registry.supports_wire_format(
        PrimitiveProviderKind::Gemini,
        ProviderPrimitiveWireFormat::GeminiOperations,
        PrimitiveStreamMode::None,
    ));

    let models = PrimitiveRequest::get(
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Models,
        ProviderPrimitiveWireFormat::GeminiModels,
        Option::<String>::None,
    );
    assert_eq!(
        models.budget_class(),
        PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost
    );
    let (_, raw_request, used_usd) = call_and_capture_gemini(models, json!({"models":[]})).await;
    assert!(raw_request.starts_with("GET /models HTTP/1.1"));
    assert!(raw_request
        .to_ascii_lowercase()
        .contains("x-goog-api-key: sk-test"));
    assert_eq!(used_usd, 0.0);

    let operation = PrimitiveRequest::get(
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Operations,
        ProviderPrimitiveWireFormat::GeminiOperations,
        Option::<String>::None,
    )
    .with_path("/operations/op_1");
    assert_eq!(
        operation.budget_class(),
        PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost
    );
    let (_, raw_request, used_usd) =
        call_and_capture_gemini(operation, json!({"name":"op_1"})).await;
    assert!(raw_request.starts_with("GET /operations/op_1 HTTP/1.1"));
    assert_eq!(used_usd, 0.0);

    let file_get = PrimitiveRequest::get(
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Files,
        ProviderPrimitiveWireFormat::GeminiFiles,
        Option::<String>::None,
    )
    .with_path("/files/file_1");
    assert_eq!(
        file_get.budget_class(),
        PrimitiveBudgetClass::UploadOrStorage
    );
    let (_, raw_request, used_usd) =
        call_and_capture_gemini(file_get, json!({"name":"files/file_1"})).await;
    assert!(raw_request.starts_with("GET /files/file_1 HTTP/1.1"));
    assert_eq!(used_usd, 0.0);

    let cache_get = PrimitiveRequest::get(
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Caches,
        ProviderPrimitiveWireFormat::GeminiCaches,
        Option::<String>::None,
    )
    .with_path("/cachedContents/cache_1");
    assert_eq!(
        cache_get.budget_class(),
        PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost
    );
    let (_, raw_request, used_usd) =
        call_and_capture_gemini(cache_get, json!({"name":"cachedContents/cache_1"})).await;
    assert!(raw_request.starts_with("GET /cachedContents/cache_1 HTTP/1.1"));
    assert_eq!(used_usd, 0.0);
}

#[tokio::test]
async fn primitive_call_extracts_anthropic_gemini_and_compatible_usage() {
    let anthropic = call_once(
        PrimitiveProviderEndpoint::new(PrimitiveProviderKind::Anthropic, "http://127.0.0.1:0"),
        PrimitiveProviderKind::Anthropic,
        PrimitiveEndpointKind::Messages,
        ProviderPrimitiveWireFormat::AnthropicMessages,
        "claude-3-5-sonnet-20241022",
        json!({
            "id":"msg_1",
            "usage": {
                "input_tokens": 20,
                "output_tokens": 4,
                "cache_read_input_tokens": 6,
                "cache_creation_input_tokens": 8
            }
        }),
    )
    .await;
    let anthropic_usage = anthropic
        .usage
        .expect("anthropic usage")
        .token_usage
        .unwrap();
    assert_eq!(anthropic_usage.prompt_tokens, 20);
    assert_eq!(anthropic_usage.completion_tokens, 4);
    assert_eq!(
        anthropic_usage
            .prompt_cache
            .and_then(|usage| usage.cache_read_input_tokens),
        Some(6)
    );

    let gemini = call_once(
        PrimitiveProviderEndpoint::new(PrimitiveProviderKind::Gemini, "http://127.0.0.1:0"),
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Messages,
        ProviderPrimitiveWireFormat::GeminiGenerateContent,
        "gemini-2.5-flash",
        json!({
            "responseId":"gemini_1",
            "usageMetadata": {
                "promptTokenCount": 30,
                "candidatesTokenCount": 7,
                "totalTokenCount": 37
            }
        }),
    )
    .await;
    let gemini_usage = gemini.usage.expect("gemini usage").token_usage.unwrap();
    assert_eq!(gemini_usage.prompt_tokens, 30);
    assert_eq!(gemini_usage.completion_tokens, 7);
    assert_eq!(gemini_usage.total_tokens, Some(37));

    let compatible = call_once(
        PrimitiveProviderEndpoint::openai_compatible("http://127.0.0.1:0"),
        PrimitiveProviderKind::OpenAiCompatible,
        PrimitiveEndpointKind::ChatCompletions,
        ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions,
        "llama-3.1-70b",
        json!({
            "id":"chatcmpl_compat",
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 3,
                "total_tokens": 15
            }
        }),
    )
    .await;
    let compatible_usage = compatible
        .usage
        .expect("compatible usage")
        .token_usage
        .unwrap();
    assert_eq!(compatible_usage.prompt_tokens, 12);
    assert_eq!(compatible_usage.completion_tokens, 3);
}

#[tokio::test]
async fn primitive_provider_error_and_local_rpm_rejection_refund_budget() {
    let (base_url, server) = spawn_server(
        500,
        Some("application/json"),
        json!({"error":{"message":"provider failed","code":"server_error"}})
            .to_string()
            .into_bytes(),
    )
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));

    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Responses,
        ProviderPrimitiveWireFormat::OpenAiResponses,
        "gpt-4o",
        json!({"model":"gpt-4o","input":"hello"}),
    );

    let err = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect_err("provider error should surface");
    assert!(matches!(err, GatewayError::PrimitiveProvider(_)));
    assert_eq!(gateway.budget_used_usd(), 0.0);
    server.await.expect("server joins").expect("server ok");

    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        json!({"id":"should_not_dispatch"}).to_string().into_bytes(),
    )
    .await;
    let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
        .primitive_endpoint(PrimitiveProviderEndpoint::new(
            PrimitiveProviderKind::OpenAi,
            base_url,
        ))
        .add_key(
            KeyConfig::new("sk-test", "rpm-zero")
                .tpm_limit(100_000)
                .rpm_limit(0),
        )
        .budget_limit_usd(1.0)
        .build()
        .expect("gateway");
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Responses,
        ProviderPrimitiveWireFormat::OpenAiResponses,
        "gpt-4o",
        json!({"model":"gpt-4o","input":"hello"}),
    );

    let err = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect_err("local RPM rejection should fail before network");
    assert!(matches!(err, GatewayError::RateLimited));
    assert_eq!(gateway.budget_used_usd(), 0.0);
    server.abort();
}

#[tokio::test]
async fn primitive_audio_speech_preserves_binary_response_with_budget_fallback() {
    let (base_url, server) = spawn_server(200, Some("audio/mpeg"), vec![0, 159, 146, 150]).await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));

    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::AudioSpeech,
        ProviderPrimitiveWireFormat::OpenAiAudioSpeech,
        "gpt-4o-mini-tts",
        json!({"model":"gpt-4o-mini-tts","input":"hello","voice":"alloy"}),
    );

    let response = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect("audio speech primitive call");

    let ResponseBody::Binary {
        data_base64,
        media_type,
    } = response.body
    else {
        panic!("expected binary response");
    };
    assert_eq!(data_base64, "AJ+Slg==");
    assert_eq!(media_type.as_deref(), Some("audio/mpeg"));
    assert!(response.usage.is_none());
    assert!(gateway.budget_used_usd() > 0.0);

    let raw_request = server.await.expect("server joins").expect("server ok");
    assert!(raw_request.starts_with("POST /audio/speech HTTP/1.1"));
}

#[tokio::test]
async fn primitive_binary_chunk_stream_preserves_audio_bytes_and_settles_budget() {
    let (base_url, server) = spawn_server(200, Some("audio/mpeg"), vec![0, 159, 146, 150]).await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::AudioSpeech,
        ProviderPrimitiveWireFormat::OpenAiAudioSpeech,
        "gpt-4o-mini-tts",
        json!({"model":"gpt-4o-mini-tts","input":"hello","voice":"alloy"}),
    )
    .with_stream(PrimitiveStreamMode::BinaryChunks);

    let mut stream = gateway
        .primitive_stream(request, CancellationToken::new())
        .await
        .expect("binary primitive stream starts");
    let mut chunks = Vec::new();
    let mut completed = false;
    while let Some(event) = stream.next().await {
        match event.expect("stream item") {
            PrimitiveStreamEvent::BinaryChunk {
                data_base64,
                media_type,
            } => {
                chunks.push(data_base64);
                assert_eq!(media_type.as_deref(), Some("audio/mpeg"));
            }
            PrimitiveStreamEvent::Completed { .. } => completed = true,
            other => panic!("unexpected event: {other:?}"),
        }
    }

    assert_eq!(chunks, vec!["AJ+Slg=="]);
    assert!(completed);
    assert!(gateway.budget_used_usd() > 0.0);
    let raw_request = server.await.expect("server joins").expect("server ok");
    assert!(raw_request.starts_with("POST /audio/speech HTTP/1.1"));
}

#[tokio::test]
async fn primitive_sse_stream_preserves_frames_and_settles_usage() {
    let body = concat!(
        "event: response.output_text.delta\n",
        "data: {\"delta\":\"hi\"}\n\n",
        "event: response.completed\n",
        "data: {\"usage\":{\"input_tokens\":10,\"output_tokens\":2,\"total_tokens\":12}}\n\n"
    );
    let (base_url, server) =
        spawn_server(200, Some("text/event-stream"), body.as_bytes().to_vec()).await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));

    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Responses,
        ProviderPrimitiveWireFormat::OpenAiResponses,
        "gpt-4o",
        json!({"model":"gpt-4o","input":"hello","stream":true}),
    )
    .with_stream(PrimitiveStreamMode::Sse);

    let mut stream = gateway
        .primitive_stream(request, CancellationToken::new())
        .await
        .expect("primitive stream starts");
    let mut frames = 0;
    let mut usage_events = 0;
    let mut completed = false;
    while let Some(event) = stream.next().await {
        match event.expect("stream item") {
            PrimitiveStreamEvent::SseFrame { .. } => frames += 1,
            PrimitiveStreamEvent::Usage { usage } => {
                usage_events += 1;
                assert_eq!(usage.token_usage.unwrap().completion_tokens, 2);
            }
            PrimitiveStreamEvent::Completed { usage } => {
                completed = true;
                assert_eq!(usage.unwrap().token_usage.unwrap().prompt_tokens, 10);
            }
            _ => {}
        }
    }

    assert_eq!(frames, 2);
    assert_eq!(usage_events, 1);
    assert!(completed);
    assert!((gateway.budget_used_usd() - 0.000080).abs() < 1e-12);
    server.await.expect("server joins").expect("server ok");
}

#[tokio::test]
async fn primitive_sse_stream_extracts_anthropic_and_gemini_usage() {
    let anthropic_usage = stream_usage_once(
        PrimitiveProviderEndpoint::anthropic(),
        PrimitiveProviderKind::Anthropic,
        PrimitiveEndpointKind::Messages,
        ProviderPrimitiveWireFormat::AnthropicMessages,
        "claude-3-5-sonnet-20241022",
        "event: message_delta\ndata: {\"usage\":{\"input_tokens\":20,\"output_tokens\":4}}\n\n",
    )
    .await;
    let anthropic_tokens = anthropic_usage.token_usage.unwrap();
    assert_eq!(anthropic_tokens.prompt_tokens, 20);
    assert_eq!(anthropic_tokens.completion_tokens, 4);

    let gemini_usage = stream_usage_once(
        PrimitiveProviderEndpoint::gemini(),
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Messages,
        ProviderPrimitiveWireFormat::GeminiStreamGenerateContent,
        "gemini-2.5-flash",
        "data: {\"usageMetadata\":{\"promptTokenCount\":30,\"candidatesTokenCount\":6,\"totalTokenCount\":36}}\n\n",
    )
    .await;
    let gemini_tokens = gemini_usage.token_usage.unwrap();
    assert_eq!(gemini_tokens.prompt_tokens, 30);
    assert_eq!(gemini_tokens.completion_tokens, 6);
    assert_eq!(gemini_tokens.total_tokens, Some(36));
}

#[tokio::test]
async fn primitive_sse_stream_cancellation_refunds_without_usage() {
    let (base_url, server) = spawn_open_sse_server().await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));

    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Responses,
        ProviderPrimitiveWireFormat::OpenAiResponses,
        "gpt-4o",
        json!({"model":"gpt-4o","input":"hello","stream":true}),
    )
    .with_stream(PrimitiveStreamMode::Sse);
    let cancel = CancellationToken::new();
    let mut stream = gateway
        .primitive_stream(request, cancel.clone())
        .await
        .expect("primitive stream starts");

    cancel.cancel();
    let err = stream
        .next()
        .await
        .expect("cancelled stream item")
        .expect_err("stream cancellation should surface");
    assert!(matches!(err, GatewayError::Cancelled));
    assert_eq!(gateway.budget_used_usd(), 0.0);
    server.abort();
}

#[tokio::test]
async fn primitive_realtime_openai_websocket_preserves_messages_and_settles_usage() {
    let (base_url, server) = spawn_websocket_server(vec![
        json!({"type":"session.created","session":{"id":"sess_1"}}),
        json!({
            "type":"response.done",
            "response":{"usage":{"input_tokens":10,"output_tokens":2,"total_tokens":12}}
        }),
    ])
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Realtime,
        ProviderPrimitiveWireFormat::OpenAiRealtime,
        "gpt-4o-realtime-preview",
        json!({"type":"session.update","session":{"modalities":["text"]}}),
    )
    .with_query("mode", "test")
    .with_header("x-realtime-test", "yes")
    .with_stream(PrimitiveStreamMode::WebSocket);

    let session = gateway
        .primitive_realtime(request, CancellationToken::new())
        .await
        .expect("realtime session succeeds");

    assert_eq!(session.provider, PrimitiveProviderKind::OpenAi);
    assert_eq!(session.stream_mode, PrimitiveStreamMode::WebSocket);
    assert!(session
        .events
        .iter()
        .any(|event| matches!(event, PrimitiveStreamEvent::WebSocketMessage { text: Some(text), .. } if text.contains("session.created"))));
    let usage = session.usage.expect("usage extracted");
    let tokens = usage.token_usage.expect("token usage");
    assert_eq!(tokens.prompt_tokens, 10);
    assert_eq!(tokens.completion_tokens, 2);
    assert!((gateway.budget_used_usd() - 0.000080).abs() < 1e-12);

    let capture = server.await.expect("server joins").expect("server ok");
    assert_eq!(capture.path_and_query, "/realtime/sessions?mode=test");
    assert_eq!(capture.authorization.as_deref(), Some("Bearer sk-test"));
    assert_eq!(capture.custom_header.as_deref(), Some("yes"));
    assert!(capture
        .initial_text
        .as_deref()
        .expect("initial text")
        .contains("session.update"));
}

#[tokio::test]
async fn primitive_realtime_gemini_live_websocket_preserves_messages_and_settles_usage() {
    let (base_url, server) = spawn_websocket_server(vec![json!({
        "serverContent": {
            "modelTurn": {"parts": [{"text": "hello"}]},
            "usageMetadata": {
                "promptTokenCount": 20,
                "candidatesTokenCount": 4,
                "totalTokenCount": 24
            }
        }
    })])
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::Gemini,
        base_url,
    ));
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Live,
        ProviderPrimitiveWireFormat::GeminiLive,
        "gemini-2.5-flash",
        json!({"setup":{"model":"models/gemini-2.5-flash"}}),
    )
    .with_path("/live")
    .with_stream(PrimitiveStreamMode::WebSocket);

    let session = gateway
        .primitive_realtime(request, CancellationToken::new())
        .await
        .expect("gemini live succeeds");

    assert_eq!(session.provider, PrimitiveProviderKind::Gemini);
    assert!(session
        .events
        .iter()
        .any(|event| matches!(event, PrimitiveStreamEvent::Usage { .. })));
    let tokens = session
        .usage
        .expect("usage extracted")
        .token_usage
        .expect("token usage");
    assert_eq!(tokens.prompt_tokens, 20);
    assert_eq!(tokens.completion_tokens, 4);
    assert_eq!(tokens.total_tokens, Some(24));
    assert!((gateway.budget_used_usd() - 0.000160).abs() < 1e-12);

    let capture = server.await.expect("server joins").expect("server ok");
    assert_eq!(capture.path_and_query, "/live");
    assert_eq!(capture.x_goog_api_key.as_deref(), Some("sk-test"));
}

#[tokio::test]
async fn primitive_realtime_settles_fallback_error_cancellation_and_webrtc_paths() {
    let (base_url, server) = spawn_websocket_server(vec![
        json!({"type":"session.created","session":{"id":"sess_2"}}),
    ])
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Realtime,
        ProviderPrimitiveWireFormat::OpenAiRealtime,
        "gpt-4o-realtime-preview",
        json!({"type":"session.update","session":{"modalities":["text"]}}),
    )
    .with_stream(PrimitiveStreamMode::WebSocket);
    let session = gateway
        .primitive_realtime(request, CancellationToken::new())
        .await
        .expect("no-usage realtime succeeds");
    assert!(session.usage.is_none());
    assert!(gateway.budget_used_usd() > 0.0);
    server.await.expect("server joins").expect("server ok");

    let (base_url, server) =
        spawn_server(500, Some("text/plain"), b"handshake failed".to_vec()).await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Realtime,
        ProviderPrimitiveWireFormat::OpenAiRealtime,
        "gpt-4o-realtime-preview",
        json!({"type":"session.update"}),
    )
    .with_stream(PrimitiveStreamMode::WebSocket);
    let err = gateway
        .primitive_realtime(request, CancellationToken::new())
        .await
        .expect_err("handshake error surfaces");
    assert!(matches!(err, GatewayError::PrimitiveProvider(_)));
    assert_eq!(gateway.budget_used_usd(), 0.0);
    server.await.expect("server joins").expect("server ok");

    let (base_url, server) = spawn_hanging_websocket_server().await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Realtime,
        ProviderPrimitiveWireFormat::OpenAiRealtime,
        "gpt-4o-realtime-preview",
        json!({"type":"session.update"}),
    )
    .with_stream(PrimitiveStreamMode::WebSocket);
    let cancel = CancellationToken::new();
    let (result, _) = tokio::join!(gateway.primitive_realtime(request, cancel.clone()), async {
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();
    });
    assert!(matches!(
        result.expect_err("cancelled"),
        GatewayError::Cancelled
    ));
    assert_eq!(gateway.budget_used_usd(), 0.0);
    server.abort();

    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        "http://127.0.0.1:9",
    ));
    let request = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Realtime,
        ProviderPrimitiveWireFormat::OpenAiRealtime,
        "gpt-4o-realtime-preview",
        json!({"type":"session.update"}),
    )
    .with_stream(PrimitiveStreamMode::WebRtc);
    let err = gateway
        .primitive_realtime(request, CancellationToken::new())
        .await
        .expect_err("webrtc remains planned");
    assert!(matches!(err, GatewayError::Protocol(_)));
    assert_eq!(gateway.budget_used_usd(), 0.0);
}

#[derive(Debug, Clone, Default)]
struct WebSocketCapture {
    path_and_query: String,
    authorization: Option<String>,
    x_goog_api_key: Option<String>,
    custom_header: Option<String>,
    initial_text: Option<String>,
}

type WebSocketTestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn spawn_websocket_server(
    messages: Vec<serde_json::Value>,
) -> (String, JoinHandle<WebSocketTestResult<WebSocketCapture>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let capture = Arc::new(Mutex::new(WebSocketCapture::default()));
    let handle = tokio::spawn(async move {
        let (socket, _) = listener.accept().await?;
        let capture_for_headers = Arc::clone(&capture);
        let mut websocket =
            accept_hdr_async(socket, move |request: &WsRequest, response: WsResponse| {
                let mut capture = capture_for_headers.lock().expect("capture lock");
                capture.path_and_query = request
                    .uri()
                    .path_and_query()
                    .map(|value| value.as_str().to_string())
                    .unwrap_or_else(|| request.uri().path().to_string());
                capture.authorization = websocket_header(request, "authorization");
                capture.x_goog_api_key = websocket_header(request, "x-goog-api-key");
                capture.custom_header = websocket_header(request, "x-realtime-test");
                Ok(response)
            })
            .await?;

        if let Some(message) = websocket.next().await {
            match message? {
                WsMessage::Text(text) => {
                    capture.lock().expect("capture lock").initial_text = Some(text.to_string());
                }
                WsMessage::Binary(data) => {
                    capture.lock().expect("capture lock").initial_text =
                        Some(format!("binary:{}", data.len()));
                }
                _ => {}
            }
        }

        for message in messages {
            websocket
                .send(WsMessage::Text(message.to_string().into()))
                .await?;
        }
        websocket.close(None).await?;
        Ok(capture.lock().expect("capture lock").clone())
    });
    (format!("http://{addr}"), handle)
}

async fn spawn_hanging_websocket_server() -> (String, JoinHandle<WebSocketTestResult<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        let (socket, _) = listener.accept().await?;
        let mut websocket =
            accept_hdr_async(socket, |_request: &WsRequest, response: WsResponse| {
                Ok(response)
            })
            .await?;
        let _ = websocket.next().await;
        tokio::time::sleep(Duration::from_secs(30)).await;
        Ok(())
    });
    (format!("http://{addr}"), handle)
}

fn websocket_header(request: &WsRequest, name: &str) -> Option<String> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn call_async_job_capture(
    endpoint: PrimitiveProviderEndpoint,
    request: PrimitiveAsyncJobRequest,
    provider_response: serde_json::Value,
) -> (omnillm::PrimitiveAsyncJobResponse, String, f64) {
    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        provider_response.to_string().into_bytes(),
    )
    .await;
    let endpoint = PrimitiveProviderEndpoint::new(endpoint.provider, base_url)
        .with_auth(endpoint.auth_scheme());
    let gateway = primitive_gateway(endpoint);
    let response = gateway
        .primitive_async_job(request, CancellationToken::new())
        .await
        .expect("primitive async job succeeds");
    let used_usd = gateway.budget_used_usd();
    let raw_request = server.await.expect("server joins").expect("server ok");
    (response, raw_request, used_usd)
}

async fn call_and_capture_gemini(
    request: PrimitiveRequest,
    provider_response: serde_json::Value,
) -> (omnillm::PrimitiveResponse, String, f64) {
    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        provider_response.to_string().into_bytes(),
    )
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::Gemini,
        base_url,
    ));
    let response = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect("primitive call succeeds");
    let used_usd = gateway.budget_used_usd();
    let raw_request = server.await.expect("server joins").expect("server ok");
    (response, raw_request, used_usd)
}

async fn call_and_capture_anthropic(
    request: PrimitiveRequest,
    provider_response: serde_json::Value,
) -> (omnillm::PrimitiveResponse, String, f64) {
    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        provider_response.to_string().into_bytes(),
    )
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::Anthropic,
        base_url,
    ));
    let response = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect("primitive call succeeds");
    let used_usd = gateway.budget_used_usd();
    let raw_request = server.await.expect("server joins").expect("server ok");
    (response, raw_request, used_usd)
}

async fn call_and_capture_openai(
    request: PrimitiveRequest,
    provider_response: serde_json::Value,
) -> (omnillm::PrimitiveResponse, String, f64) {
    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        provider_response.to_string().into_bytes(),
    )
    .await;
    let gateway = primitive_gateway(PrimitiveProviderEndpoint::new(
        PrimitiveProviderKind::OpenAi,
        base_url,
    ));
    let response = gateway
        .primitive_call(request, CancellationToken::new())
        .await
        .expect("primitive call succeeds");
    let used_usd = gateway.budget_used_usd();
    let raw_request = server.await.expect("server joins").expect("server ok");
    (response, raw_request, used_usd)
}

async fn call_once(
    endpoint: PrimitiveProviderEndpoint,
    provider: PrimitiveProviderKind,
    endpoint_kind: PrimitiveEndpointKind,
    wire_format: ProviderPrimitiveWireFormat,
    model: &str,
    provider_response: serde_json::Value,
) -> omnillm::PrimitiveResponse {
    let (base_url, server) = spawn_server(
        200,
        Some("application/json"),
        provider_response.to_string().into_bytes(),
    )
    .await;
    let endpoint = PrimitiveProviderEndpoint::new(endpoint.provider, base_url)
        .with_auth(endpoint.auth_scheme());
    let gateway = primitive_gateway(endpoint);
    let response = gateway
        .primitive_call(
            PrimitiveRequest::json(
                provider,
                endpoint_kind,
                wire_format,
                model,
                json!({"model": model, "input": "hello"}),
            ),
            CancellationToken::new(),
        )
        .await
        .expect("primitive call succeeds");
    server.await.expect("server joins").expect("server ok");
    response
}

fn primitive_gateway(endpoint: PrimitiveProviderEndpoint) -> omnillm::Gateway {
    GatewayBuilder::new(ProviderEndpoint::openai_responses())
        .primitive_endpoint(endpoint)
        .add_key(
            KeyConfig::new("sk-test", "test-key")
                .tpm_limit(100_000)
                .rpm_limit(100),
        )
        .budget_limit_usd(1.0)
        .build()
        .expect("gateway")
}

async fn stream_usage_once(
    endpoint: PrimitiveProviderEndpoint,
    provider: PrimitiveProviderKind,
    endpoint_kind: PrimitiveEndpointKind,
    wire_format: ProviderPrimitiveWireFormat,
    model: &str,
    body: &str,
) -> omnillm::PrimitiveUsageTelemetry {
    let (base_url, server) =
        spawn_server(200, Some("text/event-stream"), body.as_bytes().to_vec()).await;
    let endpoint = PrimitiveProviderEndpoint::new(endpoint.provider, base_url)
        .with_auth(endpoint.auth_scheme());
    let gateway = primitive_gateway(endpoint);
    let request = PrimitiveRequest::json(
        provider,
        endpoint_kind,
        wire_format,
        model,
        json!({"model": model, "stream": true}),
    )
    .with_stream(PrimitiveStreamMode::Sse);

    let mut stream = gateway
        .primitive_stream(request, CancellationToken::new())
        .await
        .expect("primitive stream starts");
    let mut usage_events = 0;
    let mut completed_usage = None;
    while let Some(event) = stream.next().await {
        match event.expect("stream item") {
            PrimitiveStreamEvent::Usage { usage } => {
                usage_events += 1;
                completed_usage = Some(usage);
            }
            PrimitiveStreamEvent::Completed { usage } => {
                completed_usage = usage.or(completed_usage);
            }
            _ => {}
        }
    }

    assert_eq!(usage_events, 1);
    server.await.expect("server joins").expect("server ok");
    completed_usage.expect("stream usage")
}

async fn spawn_server(
    status: u16,
    content_type: Option<&str>,
    body: Vec<u8>,
) -> (String, JoinHandle<io::Result<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let content_type = content_type.map(str::to_string);
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        let mut data = Vec::new();
        let header_end = loop {
            let mut buf = [0_u8; 1024];
            let n = socket.read(&mut buf).await?;
            if n == 0 {
                break find_header_end(&data).unwrap_or(data.len());
            }
            data.extend_from_slice(&buf[..n]);
            if let Some(pos) = find_header_end(&data) {
                break pos;
            }
        };
        let headers = String::from_utf8_lossy(&data[..header_end]).to_string();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        while data.len().saturating_sub(header_end + 4) < content_length {
            let mut buf = [0_u8; 1024];
            let n = socket.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
        }

        let reason = if status >= 400 { "ERROR" } else { "OK" };
        let mut response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        );
        if let Some(content_type) = &content_type {
            response.push_str(&format!("Content-Type: {content_type}\r\n"));
        }
        response.push_str("\r\n");
        socket.write_all(response.as_bytes()).await?;
        socket.write_all(&body).await?;
        Ok(String::from_utf8_lossy(&data).to_string())
    });
    (format!("http://{addr}"), handle)
}

async fn spawn_open_sse_server() -> (String, JoinHandle<io::Result<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        let mut data = Vec::new();
        loop {
            let mut buf = [0_u8; 1024];
            let n = socket.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            if find_header_end(&data).is_some() {
                break;
            }
        }

        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n",
            )
            .await?;
        tokio::time::sleep(Duration::from_secs(30)).await;
        Ok(String::from_utf8_lossy(&data).to_string())
    });
    (format!("http://{addr}"), handle)
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}
