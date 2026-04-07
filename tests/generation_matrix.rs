use omni_gateway::{
    emit_api_request, emit_transport_request, parse_error, parse_request, parse_response,
    parse_stream_event, transcode_error, transcode_request, transcode_response,
    transcode_stream_event, ApiRequest, BuiltinTool, CapabilitySet, GenerationConfig, LlmRequest,
    LlmStreamEvent, Message, MessageRole, ProviderProtocol, ProviderStreamFrame,
    ReasoningCapability, RequestBody, RequestItem, StructuredOutputConfig, ToolDefinition,
    VendorExtensions, WireFormat,
};
use serde_json::json;

fn generation_protocols() -> [ProviderProtocol; 4] {
    [
        ProviderProtocol::OpenAiResponses,
        ProviderProtocol::OpenAiChatCompletions,
        ProviderProtocol::ClaudeMessages,
        ProviderProtocol::GeminiGenerateContent,
    ]
}

fn generation_wire_formats() -> [WireFormat; 4] {
    [
        WireFormat::OpenAiResponses,
        WireFormat::OpenAiChatCompletions,
        WireFormat::AnthropicMessages,
        WireFormat::GeminiGenerateContent,
    ]
}

fn request_payload(protocol: ProviderProtocol) -> String {
    let tool_schema = json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    });

    match protocol {
        ProviderProtocol::OpenAiResponses => json!({
            "model": "test-model",
            "instructions": "Be terse",
            "input": [{
                "role": "user",
                "content": [{ "type": "input_text", "text": "Hello!" }]
            }],
            "tools": [{
                "type": "function",
                "name": "lookup_weather",
                "description": "Weather lookup",
                "parameters": tool_schema,
                "strict": true
            }],
            "temperature": 0.2,
            "max_output_tokens": 32
        })
        .to_string(),
        ProviderProtocol::OpenAiChatCompletions => json!({
            "model": "test-model",
            "messages": [
                { "role": "system", "content": "Be terse" },
                { "role": "user", "content": "Hello!" }
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "lookup_weather",
                    "description": "Weather lookup",
                    "parameters": tool_schema,
                    "strict": true
                }
            }],
            "temperature": 0.2,
            "max_tokens": 32
        })
        .to_string(),
        ProviderProtocol::ClaudeMessages => json!({
            "model": "test-model",
            "system": "Be terse",
            "messages": [{
                "role": "user",
                "content": [{ "type": "text", "text": "Hello!" }]
            }],
            "tools": [{
                "name": "lookup_weather",
                "description": "Weather lookup",
                "input_schema": tool_schema
            }],
            "temperature": 0.2,
            "max_tokens": 32
        })
        .to_string(),
        ProviderProtocol::GeminiGenerateContent => json!({
            "model": "test-model",
            "systemInstruction": {
                "role": "system",
                "parts": [{ "text": "Be terse" }]
            },
            "contents": [{
                "role": "user",
                "parts": [{ "text": "Hello!" }]
            }],
            "tools": [{
                "functionDeclarations": [{
                    "name": "lookup_weather",
                    "description": "Weather lookup",
                    "parameters": tool_schema
                }]
            }],
            "generationConfig": {
                "temperature": 0.2,
                "maxOutputTokens": 32
            }
        })
        .to_string(),
    }
}

fn response_payload(protocol: ProviderProtocol) -> String {
    match protocol {
        ProviderProtocol::OpenAiResponses => json!({
            "id": "resp_123",
            "model": "test-model",
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
        .to_string(),
        ProviderProtocol::OpenAiChatCompletions => json!({
            "id": "chatcmpl_123",
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "Hello back!" },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })
        .to_string(),
        ProviderProtocol::ClaudeMessages => json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "test-model",
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "Hello back!" }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        })
        .to_string(),
        ProviderProtocol::GeminiGenerateContent => json!({
            "responseId": "gemini_123",
            "modelVersion": "test-model",
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "role": "model",
                    "parts": [{ "text": "Hello back!" }]
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        })
        .to_string(),
    }
}

fn error_payload(protocol: ProviderProtocol) -> (Option<u16>, String) {
    let status = Some(400);
    let payload = match protocol {
        ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => json!({
            "error": {
                "message": "bad request",
                "type": "invalid_request_error",
                "code": "invalid_request_error"
            }
        }),
        ProviderProtocol::ClaudeMessages => json!({
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "bad request"
            }
        }),
        ProviderProtocol::GeminiGenerateContent => json!({
            "error": {
                "code": 400,
                "status": "INVALID_ARGUMENT",
                "message": "bad request"
            }
        }),
    };
    (status, payload.to_string())
}

fn text_delta_frame(protocol: ProviderProtocol) -> ProviderStreamFrame {
    match protocol {
        ProviderProtocol::OpenAiResponses => ProviderStreamFrame {
            event: Some("response.output_text.delta".into()),
            data: json!({
                "type": "response.output_text.delta",
                "delta": "Hel"
            })
            .to_string(),
        },
        ProviderProtocol::OpenAiChatCompletions => ProviderStreamFrame {
            event: None,
            data: json!({
                "choices": [{
                    "index": 0,
                    "delta": { "content": "Hel" }
                }]
            })
            .to_string(),
        },
        ProviderProtocol::ClaudeMessages => ProviderStreamFrame {
            event: Some("content_block_delta".into()),
            data: json!({
                "delta": { "type": "text_delta", "text": "Hel" }
            })
            .to_string(),
        },
        ProviderProtocol::GeminiGenerateContent => ProviderStreamFrame {
            event: None,
            data: json!({
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [{ "text": "Hel" }]
                    }
                }]
            })
            .to_string(),
        },
    }
}

fn simple_canonical_request() -> ApiRequest {
    ApiRequest::Responses(LlmRequest {
        model: "test-model".into(),
        instructions: Some("Be terse".into()),
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            "Hello!",
        ))],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            tools: vec![ToolDefinition {
                name: "lookup_weather".into(),
                description: Some("Weather lookup".into()),
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
        generation: GenerationConfig {
            max_output_tokens: Some(32),
            temperature: Some(0.2),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    })
}

fn degrading_canonical_request() -> ApiRequest {
    ApiRequest::Responses(LlmRequest {
        model: "test-model".into(),
        instructions: Some("Be terse".into()),
        input: vec![RequestItem::from(Message {
            role: MessageRole::User,
            parts: vec![omni_gateway::MessagePart::Text {
                text: "Hello!".into(),
            }],
            raw_message: Some(r#"{"role":"user","content":"Hello!"}"#.into()),
            vendor_extensions: [("message_ext".into(), json!("value"))]
                .into_iter()
                .collect(),
        })],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            tools: vec![ToolDefinition {
                name: "lookup_weather".into(),
                description: Some("Weather lookup".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    }
                }),
                strict: true,
                vendor_extensions: [("tool_ext".into(), json!(true))].into_iter().collect(),
            }],
            structured_output: Some(StructuredOutputConfig {
                name: Some("answer".into()),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "answer": { "type": "string" }
                    }
                }),
                strict: true,
            }),
            reasoning: Some(ReasoningCapability {
                effort: Some("medium".into()),
                summary: Some("auto".into()),
                vendor_extensions: [("reasoning_ext".into(), json!(true))]
                    .into_iter()
                    .collect(),
            }),
            builtin_tools: vec![BuiltinTool::WebSearch],
            ..Default::default()
        },
        generation: GenerationConfig {
            max_output_tokens: Some(32),
            temperature: Some(0.2),
            top_k: Some(5),
            presence_penalty: Some(0.8),
            frequency_penalty: Some(0.4),
            seed: Some(7),
            ..Default::default()
        },
        metadata: [("trace_id".into(), json!("trace-1"))]
            .into_iter()
            .collect(),
        vendor_extensions: [("request_ext".into(), json!("value"))]
            .into_iter()
            .collect(),
    })
}

fn assert_core_request_shape(request: &LlmRequest) {
    assert_eq!(request.model, "test-model");
    assert_eq!(
        request.normalized_instructions().as_deref(),
        Some("Be terse")
    );
    assert_eq!(request.generation.max_output_tokens, Some(32));
    assert_eq!(request.capabilities.tools.len(), 1);
    assert_eq!(request.capabilities.tools[0].name, "lookup_weather");

    let user_message = request
        .normalized_messages()
        .into_iter()
        .find(|message| message.role == MessageRole::User)
        .expect("expected a user message");
    assert_eq!(user_message.plain_text(), "Hello!");

    let temperature = request
        .generation
        .temperature
        .expect("temperature should be preserved");
    assert!((temperature - 0.2).abs() < f32::EPSILON);
}

fn assert_core_response_shape(protocol: ProviderProtocol, response: &omni_gateway::LlmResponse) {
    assert_eq!(response.provider_protocol, protocol);
    assert_eq!(response.model, "test-model");
    assert_eq!(response.content_text, "Hello back!");
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.completion_tokens, 5);
    assert_eq!(response.usage.total(), 15);
    assert!(response
        .messages
        .iter()
        .any(|message| message.role == MessageRole::Assistant
            && message.plain_text() == "Hello back!"));
}

fn assert_text_delta(event: omni_gateway::LlmStreamEvent) {
    match event {
        LlmStreamEvent::TextDelta { delta } => assert_eq!(delta, "Hel"),
        other => panic!("expected text delta event, got {other:?}"),
    }
}

#[test]
fn generation_request_transcode_matrix_preserves_core_fields() {
    for from in generation_protocols() {
        let raw = request_payload(from);
        for to in generation_protocols() {
            let transcoded = transcode_request(from, to, &raw)
                .unwrap_or_else(|error| panic!("request transcode {from:?} -> {to:?}: {error}"));
            let parsed = parse_request(to, &transcoded)
                .unwrap_or_else(|error| panic!("parse target request {to:?}: {error}"));
            assert_core_request_shape(&parsed);
        }
    }
}

#[test]
fn generation_response_transcode_matrix_preserves_core_fields() {
    for from in generation_protocols() {
        let raw = response_payload(from);
        for to in generation_protocols() {
            let transcoded = transcode_response(from, to, &raw)
                .unwrap_or_else(|error| panic!("response transcode {from:?} -> {to:?}: {error}"));
            let parsed = parse_response(to, &transcoded)
                .unwrap_or_else(|error| panic!("parse target response {to:?}: {error}"));
            assert_core_response_shape(to, &parsed);
        }
    }
}

#[test]
fn generation_error_transcode_matrix_preserves_core_fields() {
    for from in generation_protocols() {
        let (status, raw) = error_payload(from);
        for to in generation_protocols() {
            let transcoded = transcode_error(from, to, status, &raw)
                .unwrap_or_else(|error| panic!("error transcode {from:?} -> {to:?}: {error}"));
            let parsed = parse_error(to, status, &transcoded)
                .unwrap_or_else(|error| panic!("parse target error {to:?}: {error}"));
            assert_eq!(parsed.protocol, to);
            assert_eq!(parsed.status, status);
            assert_eq!(parsed.message, "bad request");
            assert!(parsed.code.is_some());
        }
    }
}

#[test]
fn generation_stream_text_delta_transcode_matrix_preserves_delta() {
    for from in generation_protocols() {
        let frame = text_delta_frame(from);
        for to in generation_protocols() {
            let transcoded = transcode_stream_event(from, to, &frame)
                .unwrap_or_else(|error| panic!("stream transcode {from:?} -> {to:?}: {error}"))
                .expect("text delta should always emit a target frame");
            let parsed = parse_stream_event(to, &transcoded)
                .unwrap_or_else(|error| panic!("parse target stream event {to:?}: {error}"))
                .expect("text delta should parse back");
            assert_text_delta(parsed);
        }
    }
}

#[test]
fn generation_parsers_preserve_raw_message_for_requests_and_responses() {
    for protocol in generation_protocols() {
        let request = parse_request(protocol, &request_payload(protocol))
            .unwrap_or_else(|error| panic!("parse request {protocol:?}: {error}"));
        assert!(request
            .normalized_messages()
            .into_iter()
            .any(|message| { message.role == MessageRole::User && message.raw_message.is_some() }));

        let response = parse_response(protocol, &response_payload(protocol))
            .unwrap_or_else(|error| panic!("parse response {protocol:?}: {error}"));
        assert!(response.messages.iter().any(|message| {
            message.role == MessageRole::Assistant && message.raw_message.is_some()
        }));
    }
}

#[test]
fn generation_transport_requests_use_expected_paths_and_bridge_flags() {
    let request = simple_canonical_request();

    for wire_format in generation_wire_formats() {
        let report = emit_transport_request(wire_format, &request)
            .unwrap_or_else(|error| panic!("emit transport request {wire_format:?}: {error}"));

        match wire_format {
            WireFormat::OpenAiResponses => {
                assert!(!report.bridged);
                assert_eq!(report.value.path, "/responses");
            }
            WireFormat::OpenAiChatCompletions => {
                assert!(report.bridged);
                assert_eq!(report.value.path, "/chat/completions");
            }
            WireFormat::AnthropicMessages => {
                assert!(report.bridged);
                assert_eq!(report.value.path, "/messages");
            }
            WireFormat::GeminiGenerateContent => {
                assert!(report.bridged);
                assert_eq!(report.value.path, "/models/test-model:generateContent");
            }
            _ => unreachable!("only generation wire formats are tested here"),
        }

        assert!(!report.lossy);
        let RequestBody::Json { value } = report.value.body else {
            panic!("generation transport request should always be JSON");
        };
        match wire_format {
            WireFormat::GeminiGenerateContent => {
                assert!(value.get("model").is_none());
                assert_eq!(value["contents"][0]["parts"][0]["text"], "Hello!");
            }
            WireFormat::OpenAiResponses => {
                assert_eq!(value["model"], "test-model");
                assert_eq!(value["input"][0]["content"][0]["text"], "Hello!");
            }
            WireFormat::OpenAiChatCompletions => {
                assert_eq!(value["model"], "test-model");
                assert_eq!(value["messages"][1]["content"], "Hello!");
            }
            WireFormat::AnthropicMessages => {
                assert_eq!(value["model"], "test-model");
                assert_eq!(value["messages"][0]["content"][0]["text"], "Hello!");
            }
            _ => unreachable!("only generation wire formats are tested here"),
        }
    }
}

#[test]
fn generation_loss_reports_are_explicit_for_claude_and_gemini() {
    let request = degrading_canonical_request();

    let claude = emit_api_request(WireFormat::AnthropicMessages, &request)
        .expect("emit claude request with loss reporting");
    assert!(claude.lossy);
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("structured output")));
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("reasoning settings")));
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("top_k")));
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("presence_penalty")));
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("frequency_penalty")));
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("seed")));
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("metadata")));
    assert!(claude
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("vendor_extensions and raw_message")));

    let gemini = emit_api_request(WireFormat::GeminiGenerateContent, &request)
        .expect("emit gemini request with loss reporting");
    assert!(gemini.lossy);
    assert!(gemini
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("code_execution builtin tools")));
    assert!(gemini
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("reasoning settings")));
    assert!(gemini
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("presence_penalty")));
    assert!(gemini
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("frequency_penalty")));
    assert!(gemini
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("metadata")));
    assert!(gemini
        .loss_reasons
        .iter()
        .any(|reason| reason.contains("vendor_extensions and raw_message")));
}
