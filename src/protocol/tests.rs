use super::*;
use serde_json::{json, Value};

use crate::types::{
    CacheBreakpoint, CapabilitySet, GenerationConfig, LlmRequest, LlmStreamEvent, Message,
    MessagePart, MessageRole, PromptCacheKey, PromptCachePolicy, PromptCacheRetention, RequestItem,
    ResponseItem, ToolDefinition, VendorExtensions,
};

#[test]
fn endpoint_protocol_parses_official_and_compat_aliases() {
    assert_eq!(
        "openai_chat_completions"
            .parse::<EndpointProtocol>()
            .expect("official parse"),
        EndpointProtocol::OpenAiChatCompletions
    );
    assert_eq!(
        "open_ai_chat_completions_compat"
            .parse::<EndpointProtocol>()
            .expect("compat parse"),
        EndpointProtocol::OpenAiChatCompletionsCompat
    );
    assert_eq!(
        "anthropic_messages"
            .parse::<EndpointProtocol>()
            .expect("anthropic alias"),
        EndpointProtocol::ClaudeMessages
    );
}

#[test]
fn provider_endpoint_uses_compat_base_url_as_final_request_url() {
    let endpoint = ProviderEndpoint::new(
        EndpointProtocol::OpenAiChatCompletionsCompat,
        "https://aidp.bytedance.net/api/modelhub/online/v2/crawl",
    );

    assert_eq!(
        endpoint.request_url("openai_qwen3.6-plus", true),
        "https://aidp.bytedance.net/api/modelhub/online/v2/crawl",
    );
    assert_eq!(
        endpoint.wire_protocol(),
        ProviderProtocol::OpenAiChatCompletions
    );
}

#[test]
fn provider_endpoint_keeps_official_path_derivation() {
    let endpoint = ProviderEndpoint::new(
        EndpointProtocol::OpenAiChatCompletions,
        "https://api.openai.com/v1",
    );

    assert_eq!(
        endpoint.request_url("gpt-4.1-mini", false),
        "https://api.openai.com/v1/chat/completions"
    );
}

#[test]
fn transcode_chat_request_to_responses_request() {
    let chat = json!({
        "model": "gpt-4.1-mini",
        "messages": [{ "role": "user", "content": "Hello!" }],
        "max_tokens": 32
    });

    let transcoded = transcode_request(
        ProviderProtocol::OpenAiChatCompletions,
        ProviderProtocol::OpenAiResponses,
        &chat.to_string(),
    )
    .expect("transcode request");
    let body: Value = serde_json::from_str(&transcoded).expect("parse transcoded request");

    assert_eq!(body["model"], "gpt-4.1-mini");
    assert_eq!(body["input"][0]["role"], "user");
    assert_eq!(body["input"][0]["content"][0]["text"], "Hello!");
    assert_eq!(body["max_output_tokens"], 32);
}

#[test]
fn emit_openai_responses_request_uses_responses_tool_shape() {
    let request = LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: None,
        input: vec![RequestItem::from(Message::text(MessageRole::User, "Hello"))],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            tools: vec![ToolDefinition {
                name: "lookup_weather".into(),
                description: Some("Get weather".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    }
                }),
                strict: true,
                vendor_extensions: VendorExtensions::new(),
            }],
            ..Default::default()
        },
        generation: GenerationConfig::default(),
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let raw =
        emit_request(ProviderProtocol::OpenAiResponses, &request).expect("emit responses request");
    let body: Value = serde_json::from_str(&raw).expect("parse emitted body");

    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["name"], "lookup_weather");
    assert!(body["tools"][0].get("function").is_none());
    assert_eq!(body["tools"][0]["strict"], true);
}

#[test]
fn emit_openai_chat_request_uses_content_part_arrays_for_plain_text_messages() {
    let request = LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: None,
        input: vec![RequestItem::from(Message::text(MessageRole::User, "Hello"))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig::default(),
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let raw =
        emit_request(ProviderProtocol::OpenAiChatCompletions, &request).expect("emit chat request");
    let body: Value = serde_json::from_str(&raw).expect("parse emitted body");

    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"][0]["type"], "text");
    assert_eq!(body["messages"][0]["content"][0]["text"], "Hello");
}

#[test]
fn openai_chat_request_round_trips_top_level_vendor_extensions() {
    let raw = json!({
        "model": "openai_qwen3.5-plus",
        "messages": [{
            "role": "user",
            "content": "Say hello in Chinese."
        }],
        "enable_thinking": false,
        "stream": true
    });

    let request = parse_request(ProviderProtocol::OpenAiChatCompletions, &raw.to_string())
        .expect("parse chat request");

    assert_eq!(
        request.vendor_extensions.get("enable_thinking"),
        Some(&Value::Bool(false))
    );
    assert!(request.vendor_extensions.get("stream").is_none());

    let emitted =
        emit_request(ProviderProtocol::OpenAiChatCompletions, &request).expect("emit chat request");
    let body: Value = serde_json::from_str(&emitted).expect("parse emitted body");

    assert_eq!(body["enable_thinking"], false);
    assert!(body.get("stream").is_none());
}

#[test]
fn emit_openai_responses_request_includes_top_level_vendor_extensions() {
    let request = LlmRequest {
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
    };

    let emitted =
        emit_request(ProviderProtocol::OpenAiResponses, &request).expect("emit responses request");
    let body: Value = serde_json::from_str(&emitted).expect("parse emitted body");

    assert_eq!(body["enable_thinking"], false);
}

#[test]
fn parse_openai_responses_response_extracts_usage_and_message() {
    let raw = json!({
        "id": "resp_123",
        "model": "gpt-4.1-mini",
        "status": "stop",
        "output_text": "Hello back!",
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
    });

    let response = parse_response(ProviderProtocol::OpenAiResponses, &raw.to_string())
        .expect("parse response");

    assert_eq!(response.content_text, "Hello back!");
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.completion_tokens, 5);
    assert_eq!(response.usage.total(), 15);
    assert_eq!(response.response_id.as_deref(), Some("resp_123"));
    assert!(matches!(
        response.output.first(),
        Some(ResponseItem::Message { .. })
    ));
}

#[test]
fn transcode_claude_error_to_openai_error() {
    let raw = json!({
        "type": "error",
        "error": {
            "type": "invalid_request_error",
            "message": "bad request"
        }
    });

    let transcoded = transcode_error(
        ProviderProtocol::ClaudeMessages,
        ProviderProtocol::OpenAiResponses,
        Some(400),
        &raw.to_string(),
    )
    .expect("transcode error");
    let body: Value = serde_json::from_str(&transcoded).expect("parse error body");

    assert_eq!(body["error"]["message"], "bad request");
    assert_eq!(body["error"]["code"], "invalid_request_error");
}

#[test]
fn transcode_stream_event_openai_chat_to_claude() {
    let frame = ProviderStreamFrame {
        event: None,
        data: json!({
            "choices": [{
                "index": 0,
                "delta": { "content": "Hel" }
            }]
        })
        .to_string(),
    };

    let transcoded = transcode_stream_event(
        ProviderProtocol::OpenAiChatCompletions,
        ProviderProtocol::ClaudeMessages,
        &frame,
    )
    .expect("transcode stream event")
    .expect("expected frame");

    assert_eq!(transcoded.event.as_deref(), Some("content_block_delta"));
    let body: Value = serde_json::from_str(&transcoded.data).expect("parse frame body");
    assert_eq!(body["delta"]["text"], "Hel");
}

#[test]
fn parse_openai_chat_stream_events_preserves_started_and_text_delta() {
    let frame = ProviderStreamFrame {
        event: None,
        data: json!({
            "id": "chatcmpl_123",
            "model": "gpt-4.1-mini",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "Hel"
                }
            }]
        })
        .to_string(),
    };

    let events = parse_stream_events(ProviderProtocol::OpenAiChatCompletions, &frame)
        .expect("parse stream events");

    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        LlmStreamEvent::ResponseStarted {
            response_id,
            model,
            provider_protocol,
        } if response_id.as_deref() == Some("chatcmpl_123")
            && model == "gpt-4.1-mini"
            && *provider_protocol == ProviderProtocol::OpenAiChatCompletions
    ));
    assert!(matches!(
        &events[1],
        LlmStreamEvent::TextDelta { delta } if delta == "Hel"
    ));

    let primary = parse_stream_event(ProviderProtocol::OpenAiChatCompletions, &frame)
        .expect("parse primary stream event")
        .expect("expected primary event");
    assert!(matches!(
        primary,
        LlmStreamEvent::TextDelta { delta } if delta == "Hel"
    ));
}

#[test]
fn parse_openai_chat_usage_chunk_without_choices() {
    let frame = ProviderStreamFrame {
        event: None,
        data: json!({
            "id": "chatcmpl_123",
            "choices": [],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18
            }
        })
        .to_string(),
    };

    let events = parse_stream_events(ProviderProtocol::OpenAiChatCompletions, &frame)
        .expect("parse stream events");
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0],
        LlmStreamEvent::Usage { usage }
            if usage.prompt_tokens == 11
                && usage.completion_tokens == 7
                && usage.total_tokens == Some(18)
    ));

    let primary = parse_stream_event(ProviderProtocol::OpenAiChatCompletions, &frame)
        .expect("parse primary stream event")
        .expect("expected usage event");
    assert!(matches!(
        primary,
        LlmStreamEvent::Usage { usage }
            if usage.prompt_tokens == 11
                && usage.completion_tokens == 7
                && usage.total_tokens == Some(18)
    ));
}

#[test]
fn transcode_stream_events_openai_chat_to_claude_preserves_started_and_text_delta() {
    let frame = ProviderStreamFrame {
        event: None,
        data: json!({
            "id": "chatcmpl_123",
            "model": "gpt-4.1-mini",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "Hel"
                }
            }]
        })
        .to_string(),
    };

    let transcoded = transcode_stream_events(
        ProviderProtocol::OpenAiChatCompletions,
        ProviderProtocol::ClaudeMessages,
        &frame,
    )
    .expect("transcode stream events");

    assert_eq!(transcoded.len(), 2);
    assert_eq!(transcoded[0].event.as_deref(), Some("message_start"));
    assert_eq!(transcoded[1].event.as_deref(), Some("content_block_delta"));

    let body: Value =
        serde_json::from_str(&transcoded[1].data).expect("parse content block frame body");
    assert_eq!(body["delta"]["text"], "Hel");
}

#[test]
fn take_sse_frames_splits_multiple_events() {
    let mut buffer = "event: message_start\ndata: {\"foo\":1}\n\ndata: [DONE]\n\n".to_string();
    let frames = take_sse_frames(&mut buffer);

    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].event.as_deref(), Some("message_start"));
    assert_eq!(frames[0].data, "{\"foo\":1}");
    assert_eq!(frames[1].event, None);
    assert_eq!(frames[1].data, "[DONE]");
    assert!(buffer.is_empty());
}

#[test]
fn parse_gemini_request_with_schema_and_function_tools() {
    let raw = json!({
        "model": "gemini-2.5-pro",
        "systemInstruction": {
            "role": "system",
            "parts": [{ "text": "Be strict JSON." }]
        },
        "contents": [{
            "role": "user",
            "parts": [{ "text": "Return JSON" }]
        }],
        "tools": [{
            "functionDeclarations": [{
                "name": "lookup_weather",
                "description": "Weather lookup",
                "parameters": { "type": "object", "properties": { "city": { "type": "string" } } }
            }]
        }],
        "generationConfig": {
            "maxOutputTokens": 64,
            "responseMimeType": "application/json",
            "responseSchema": {
                "type": "object",
                "properties": { "answer": { "type": "string" } }
            }
        }
    });

    let request = parse_request(ProviderProtocol::GeminiGenerateContent, &raw.to_string())
        .expect("parse gemini request");

    assert_eq!(request.model, "gemini-2.5-pro");
    assert_eq!(request.instructions.as_deref(), Some("Be strict JSON."));
    assert_eq!(request.generation.max_output_tokens, Some(64));
    assert_eq!(request.capabilities.tools.len(), 1);
    assert!(request.capabilities.structured_output.is_some());
}

#[test]
fn emit_and_parse_gemini_request_round_trips_model_for_transcoding() {
    let request = LlmRequest {
        model: "gemini-2.5-pro".into(),
        instructions: Some("Return JSON.".into()),
        input: vec![RequestItem::from(Message::text(MessageRole::User, "ping"))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig {
            max_output_tokens: Some(16),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let raw = emit_request(ProviderProtocol::GeminiGenerateContent, &request)
        .expect("emit gemini request");
    let parsed =
        parse_request(ProviderProtocol::GeminiGenerateContent, &raw).expect("parse gemini request");

    assert_eq!(parsed.model, "gemini-2.5-pro");
    assert_eq!(parsed.generation.max_output_tokens, Some(16));
}

fn prompt_cache_request(policy: PromptCachePolicy) -> LlmRequest {
    LlmRequest {
        model: "gpt-5.4".into(),
        instructions: Some("Keep stable instructions.".into()),
        input: vec![RequestItem::from(Message::text(MessageRole::User, "Hello"))],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            prompt_cache: Some(policy),
            ..Default::default()
        },
        generation: GenerationConfig::default(),
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    }
}

#[test]
fn parse_openai_responses_response_reads_cached_input_tokens() {
    let raw = json!({
        "id": "resp_123",
        "model": "gpt-5.4",
        "status": "completed",
        "output_text": "ok",
        "usage": {
            "input_tokens": 1200,
            "input_tokens_details": { "cached_tokens": 1024 },
            "output_tokens": 10,
            "total_tokens": 1210
        }
    })
    .to_string();

    let response = parse_response(ProviderProtocol::OpenAiResponses, &raw)
        .expect("parse openai responses response");

    assert_eq!(
        response
            .usage
            .prompt_cache
            .expect("prompt cache usage")
            .cached_input_tokens,
        Some(1024)
    );
}

#[test]
fn parse_openai_chat_response_and_stream_read_cached_input_tokens() {
    let raw = json!({
        "id": "chatcmpl_123",
        "model": "gpt-5.4",
        "choices": [{
            "index": 0,
            "finish_reason": "stop",
            "message": { "role": "assistant", "content": "ok" }
        }],
        "usage": {
            "prompt_tokens": 1200,
            "prompt_tokens_details": { "cached_tokens": 768 },
            "completion_tokens": 10,
            "total_tokens": 1210
        }
    })
    .to_string();
    let response = parse_response(ProviderProtocol::OpenAiChatCompletions, &raw)
        .expect("parse openai chat response");
    assert_eq!(
        response
            .usage
            .prompt_cache
            .as_ref()
            .and_then(|usage| usage.cached_input_tokens),
        Some(768)
    );

    let frame = ProviderStreamFrame {
        event: None,
        data: json!({
            "usage": {
                "prompt_tokens": 1200,
                "prompt_tokens_details": { "cached_tokens": 512 },
                "completion_tokens": 10,
                "total_tokens": 1210
            }
        })
        .to_string(),
    };
    let events = parse_stream_events(ProviderProtocol::OpenAiChatCompletions, &frame)
        .expect("parse usage stream chunk");
    assert!(matches!(
        &events[0],
        LlmStreamEvent::Usage { usage }
            if usage.prompt_cache.as_ref().and_then(|cache| cache.cached_input_tokens) == Some(512)
    ));
}

#[test]
fn parse_claude_response_reads_cache_usage_tokens() {
    let raw = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet-4-5",
        "content": [{ "type": "text", "text": "ok" }],
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 50,
            "output_tokens": 10,
            "cache_read_input_tokens": 1000,
            "cache_creation_input_tokens": 2000,
            "cache_creation": {
                "ephemeral_5m_input_tokens": 300,
                "ephemeral_1h_input_tokens": 400
            }
        }
    })
    .to_string();

    let response =
        parse_response(ProviderProtocol::ClaudeMessages, &raw).expect("parse claude response");
    let prompt_cache = response.usage.prompt_cache.expect("prompt cache usage");
    assert_eq!(prompt_cache.cache_read_input_tokens, Some(1000));
    assert_eq!(prompt_cache.cache_creation_input_tokens, Some(2000));
    assert_eq!(prompt_cache.cache_creation_short_input_tokens, Some(300));
    assert_eq!(prompt_cache.cache_creation_long_input_tokens, Some(400));
}

#[test]
fn emit_openai_prompt_cache_fields_from_typed_policy() {
    let mut request = prompt_cache_request(PromptCachePolicy::BestEffort {
        key: Some(PromptCacheKey::Explicit {
            value: "tenant-a".into(),
        }),
        retention: PromptCacheRetention::Long,
        breakpoint: CacheBreakpoint::Auto,
        vendor_extensions: VendorExtensions::new(),
    });
    request.vendor_extensions.insert(
        "prompt_cache_retention".into(),
        Value::String("in_memory".into()),
    );

    let raw = emit_request(ProviderProtocol::OpenAiResponses, &request)
        .expect("emit openai responses request");
    let body: Value = serde_json::from_str(&raw).expect("parse emitted request");

    assert_eq!(body["prompt_cache_key"], "tenant-a");
    assert_eq!(body["prompt_cache_retention"], "24h");
}

#[test]
fn emit_openai_required_prompt_cache_rejects_explicit_breakpoint() {
    let request = prompt_cache_request(PromptCachePolicy::Required {
        key: None,
        retention: PromptCacheRetention::ProviderDefault,
        breakpoint: CacheBreakpoint::EndOfInstructions,
        vendor_extensions: VendorExtensions::new(),
    });

    let err = emit_request(ProviderProtocol::OpenAiResponses, &request)
        .expect_err("required explicit breakpoint should fail");
    assert!(matches!(err, ProtocolError::UnsupportedFeature(_)));
}

#[test]
fn emit_claude_prompt_cache_on_system_and_content_block() {
    let request = prompt_cache_request(PromptCachePolicy::Required {
        key: None,
        retention: PromptCacheRetention::Long,
        breakpoint: CacheBreakpoint::EndOfInstructions,
        vendor_extensions: VendorExtensions::new(),
    });
    let raw =
        emit_request(ProviderProtocol::ClaudeMessages, &request).expect("emit claude request");
    let body: Value = serde_json::from_str(&raw).expect("parse emitted claude request");
    assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
    assert_eq!(body["system"][0]["cache_control"]["ttl"], "1h");

    let block_request = LlmRequest {
        capabilities: CapabilitySet {
            prompt_cache: Some(PromptCachePolicy::Required {
                key: None,
                retention: PromptCacheRetention::ProviderDefault,
                breakpoint: CacheBreakpoint::EndOfContentBlock {
                    message_index: 0,
                    part_index: 1,
                },
                vendor_extensions: VendorExtensions::new(),
            }),
            ..Default::default()
        },
        instructions: None,
        input: vec![RequestItem::from(Message {
            role: MessageRole::User,
            parts: vec![
                MessagePart::Text {
                    text: "stable a".into(),
                },
                MessagePart::Text {
                    text: "stable b".into(),
                },
            ],
            raw_message: None,
            vendor_extensions: VendorExtensions::new(),
        })],
        ..prompt_cache_request(PromptCachePolicy::best_effort())
    };
    let raw = emit_request(ProviderProtocol::ClaudeMessages, &block_request)
        .expect("emit claude content block cache request");
    let body: Value = serde_json::from_str(&raw).expect("parse emitted claude request");
    assert!(body["messages"][0]["content"][0]["cache_control"].is_null());
    assert_eq!(
        body["messages"][0]["content"][1]["cache_control"]["type"],
        "ephemeral"
    );
}

#[test]
fn emit_claude_required_prompt_cache_rejects_missing_breakpoint_target() {
    let request = LlmRequest {
        instructions: None,
        capabilities: CapabilitySet {
            prompt_cache: Some(PromptCachePolicy::Required {
                key: None,
                retention: PromptCacheRetention::ProviderDefault,
                breakpoint: CacheBreakpoint::EndOfTools,
                vendor_extensions: VendorExtensions::new(),
            }),
            ..Default::default()
        },
        ..prompt_cache_request(PromptCachePolicy::best_effort())
    };

    let err = emit_request(ProviderProtocol::ClaudeMessages, &request)
        .expect_err("required missing tools breakpoint should fail");
    assert!(matches!(err, ProtocolError::UnsupportedFeature(_)));
}
