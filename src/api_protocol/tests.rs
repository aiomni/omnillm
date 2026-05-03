use super::*;
use serde_json::{json, Value};

use crate::api::{
    ApiRequest, ApiResponse, AudioInput, AudioTranscriptionRequest, EmbeddingInput,
    EmbeddingRequest, RequestBody, ResponseBody, TransportResponse, WireFormat,
};
use crate::types::{
    BuiltinTool, CacheBreakpoint, CapabilitySet, GenerationConfig, LlmRequest, Message,
    MessageRole, PromptCacheKey, PromptCachePolicy, PromptCacheRetention, RequestItem,
    ToolDefinition, VendorExtensions,
};

#[test]
fn emit_transport_request_for_embeddings_uses_expected_path() {
    let request = ApiRequest::Embeddings(EmbeddingRequest {
        model: "text-embedding-3-small".into(),
        input: vec![EmbeddingInput::Text {
            text: "hello".into(),
        }],
        dimensions: None,
        encoding_format: None,
        user: None,
        vendor_extensions: VendorExtensions::new(),
    });

    let report =
        emit_transport_request(WireFormat::OpenAiEmbeddings, &request).expect("emit transport");

    assert_eq!(report.value.path, "/embeddings");
    let RequestBody::Json { value } = report.value.body else {
        panic!("expected json body");
    };
    assert_eq!(value["input"], "hello");
}

#[test]
fn emit_api_request_degrades_generation_capabilities_for_chat() {
    let request = ApiRequest::Responses(LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: Some("be concise".into()),
        input: vec![RequestItem::from(Message::text(MessageRole::User, "hi"))],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            tools: vec![ToolDefinition {
                name: "lookup_weather".into(),
                description: None,
                input_schema: json!({"type":"object"}),
                strict: false,
                vendor_extensions: VendorExtensions::new(),
            }],
            builtin_tools: vec![BuiltinTool::WebSearch],
            reasoning: Some(crate::types::ReasoningCapability {
                effort: Some("medium".into()),
                summary: None,
                vendor_extensions: VendorExtensions::new(),
            }),
            ..Default::default()
        },
        generation: GenerationConfig::default(),
        metadata: [("trace_id".into(), Value::String("abc".into()))]
            .into_iter()
            .collect(),
        vendor_extensions: VendorExtensions::new(),
    });

    let report =
        emit_api_request(WireFormat::OpenAiChatCompletions, &request).expect("emit request");

    assert!(report.bridged);
    assert!(report.lossy);
    assert!(report
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("builtin tools")));
    assert!(report
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("reasoning settings")));
    assert!(report
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("metadata")));
}

#[test]
fn emit_transport_request_for_chat_keeps_top_level_vendor_extensions() {
    let request = ApiRequest::Responses(LlmRequest {
        model: "openai_qwen3.5-plus".into(),
        instructions: None,
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            "Say hello in Chinese.",
        ))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig::default(),
        metadata: Default::default(),
        vendor_extensions: [("enable_thinking".into(), Value::Bool(false))]
            .into_iter()
            .collect(),
    });

    let report = emit_transport_request(WireFormat::OpenAiChatCompletions, &request)
        .expect("emit transport request");

    assert!(report.bridged);
    assert!(!report.lossy);
    let RequestBody::Json { value } = report.value.body else {
        panic!("expected json body");
    };
    assert_eq!(value["enable_thinking"], false);
}

fn prompt_cache_api_request(policy: PromptCachePolicy) -> ApiRequest {
    ApiRequest::Responses(LlmRequest {
        model: "gpt-5.4".into(),
        instructions: Some("Stable instructions".into()),
        input: vec![RequestItem::from(Message::text(MessageRole::User, "hi"))],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            prompt_cache: Some(policy),
            ..Default::default()
        },
        generation: GenerationConfig::default(),
        metadata: Default::default(),
        vendor_extensions: VendorExtensions::new(),
    })
}

#[test]
fn best_effort_prompt_cache_reports_loss_for_gemini() {
    let request = prompt_cache_api_request(PromptCachePolicy::BestEffort {
        key: Some(PromptCacheKey::Explicit {
            value: "tenant-a".into(),
        }),
        retention: PromptCacheRetention::Long,
        breakpoint: CacheBreakpoint::Auto,
        vendor_extensions: VendorExtensions::new(),
    });

    let report = emit_api_request(WireFormat::GeminiGenerateContent, &request)
        .expect("best-effort prompt cache should degrade for gemini");

    assert!(report.bridged);
    assert!(report.lossy);
    assert!(report
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("prompt cache policy is dropped")));
    let body: Value = serde_json::from_str(&report.value).expect("parse gemini body");
    assert!(body.get("prompt_cache_key").is_none());
}

#[test]
fn required_prompt_cache_errors_for_gemini() {
    let request = prompt_cache_api_request(PromptCachePolicy::Required {
        key: None,
        retention: PromptCacheRetention::ProviderDefault,
        breakpoint: CacheBreakpoint::Auto,
        vendor_extensions: VendorExtensions::new(),
    });

    let err = emit_api_request(WireFormat::GeminiGenerateContent, &request)
        .expect_err("required prompt cache should fail for gemini");
    assert!(matches!(err, ApiProtocolError::UnsupportedFeature { .. }));
}

#[test]
fn best_effort_openai_breakpoint_is_lossy_but_emits_supported_fields() {
    let request = prompt_cache_api_request(PromptCachePolicy::BestEffort {
        key: Some(PromptCacheKey::Explicit {
            value: "tenant-a".into(),
        }),
        retention: PromptCacheRetention::Short,
        breakpoint: CacheBreakpoint::EndOfInstructions,
        vendor_extensions: VendorExtensions::new(),
    });

    let report = emit_api_request(WireFormat::OpenAiResponses, &request)
        .expect("best-effort openai prompt cache should emit partial support");
    assert!(report.lossy);
    assert!(report
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("prompt cache breakpoint")));
    let body: Value = serde_json::from_str(&report.value).expect("parse openai body");
    assert_eq!(body["prompt_cache_key"], "tenant-a");
    assert_eq!(body["prompt_cache_retention"], "in_memory");
}

#[test]
fn required_claude_prompt_cache_key_errors_before_transport() {
    let request = prompt_cache_api_request(PromptCachePolicy::Required {
        key: Some(PromptCacheKey::Explicit {
            value: "tenant-a".into(),
        }),
        retention: PromptCacheRetention::ProviderDefault,
        breakpoint: CacheBreakpoint::EndOfInstructions,
        vendor_extensions: VendorExtensions::new(),
    });

    let err = emit_transport_request(WireFormat::AnthropicMessages, &request)
        .expect_err("required claude cache key should fail");
    assert!(matches!(err, ApiProtocolError::UnsupportedFeature { .. }));
}
#[test]
fn parse_transport_response_for_audio_speech_reads_binary_payload() {
    let response = TransportResponse {
        status: 200,
        headers: Default::default(),
        content_type: Some("audio/mpeg".into()),
        body: ResponseBody::Binary {
            data_base64: "ZmFrZQ==".into(),
            media_type: None,
        },
    };

    let report = parse_transport_response(WireFormat::OpenAiAudioSpeech, &response)
        .expect("parse transport response");

    let ApiResponse::AudioSpeech(audio) = report.value else {
        panic!("expected audio speech response");
    };
    assert_eq!(audio.data_base64, "ZmFrZQ==");
    assert_eq!(audio.media_type.as_deref(), Some("audio/mpeg"));
}

#[test]
fn transcode_api_response_from_responses_to_chat_keeps_text() {
    let raw = json!({
        "id": "resp_123",
        "model": "gpt-4.1-mini",
        "status": "stop",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "Hello back!" }]
        }],
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5,
            "total_tokens": 15
        }
    })
    .to_string();

    let report = transcode_api_response(
        WireFormat::OpenAiResponses,
        WireFormat::OpenAiChatCompletions,
        &raw,
    )
    .expect("transcode response");

    let body: Value = serde_json::from_str(&report.value).expect("parse response");
    assert_eq!(body["choices"][0]["message"]["content"][0]["type"], "text");
    assert_eq!(
        body["choices"][0]["message"]["content"][0]["text"],
        "Hello back!"
    );
}

#[test]
fn parse_and_emit_rerank_round_trips_documents() {
    let raw = json!({
        "model": "rerank-v1",
        "query": "rust",
        "documents": ["Rust Book", {"title":"Cargo"}],
        "top_n": 2,
        "return_documents": true
    })
    .to_string();

    let parsed = parse_api_request(WireFormat::OpenAiRerank, &raw).expect("parse rerank request");
    let ApiRequest::Rerank(request) = &parsed.value else {
        panic!("expected rerank request");
    };
    assert_eq!(request.documents.len(), 2);

    let emitted = emit_api_request(WireFormat::OpenAiRerank, &parsed.value).expect("emit rerank");
    let body: Value = serde_json::from_str(&emitted.value).expect("parse emitted rerank");
    assert_eq!(body["documents"][0], "Rust Book");
    assert_eq!(body["documents"][1]["title"], "Cargo");
}

#[test]
fn emit_audio_transcription_transport_uses_multipart() {
    let request = ApiRequest::AudioTranscriptions(AudioTranscriptionRequest {
        model: "whisper-1".into(),
        audio: AudioInput::File {
            filename: "clip.wav".into(),
            data_base64: "ZmFrZQ==".into(),
            media_type: Some("audio/wav".into()),
        },
        prompt: None,
        response_format: Some("verbose_json".into()),
        language: Some("en".into()),
        temperature: Some(0.0),
        timestamp_granularities: vec!["word".into()],
        vendor_extensions: VendorExtensions::new(),
    });

    let report = emit_transport_request(WireFormat::OpenAiAudioTranscriptions, &request)
        .expect("emit audio transport");

    let RequestBody::Multipart { fields } = report.value.body else {
        panic!("expected multipart body");
    };
    assert!(fields.iter().any(|field| field.name == "file"));
    assert!(fields
        .iter()
        .any(|field| field.name == "timestamp_granularities[]"));
}
