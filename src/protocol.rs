//! Provider protocol definitions and canonical/raw conversion helpers.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use thiserror::Error;

use crate::error::ProviderError;
use crate::types::{
    BuiltinTool, CapabilitySet, FinishReason, GenerationConfig, LlmRequest, LlmResponse,
    LlmStreamEvent, Message, MessagePart, MessageRole, OutputModality, ReasoningCapability,
    RequestItem, ResponseItem, StructuredOutputConfig, TokenUsage, ToolCallPart, ToolDefinition,
    ToolResultPart, VendorExtensions,
};

/// Low-level upstream generation wire protocols used by the codec/transcoder
/// layer.
///
/// These names follow upstream endpoint families, not runtime configuration
/// presets. For example, `ClaudeMessages` refers to Anthropic's `/messages`
/// API, and `GeminiGenerateContent` refers to Gemini's `generateContent` API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProtocol {
    OpenAiResponses,
    OpenAiChatCompletions,
    ClaudeMessages,
    GeminiGenerateContent,
}

/// Runtime endpoint profiles used by [`ProviderEndpoint`].
///
/// Official variants derive request URLs from a base host/prefix. `Compat`
/// variants reuse the same wire protocol against a non-standard endpoint and
/// treat `base_url` as the final request URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointProtocol {
    OpenAiResponses,
    OpenAiChatCompletions,
    ClaudeMessages,
    GeminiGenerateContent,
    OpenAiResponsesCompat,
    OpenAiChatCompletionsCompat,
    ClaudeMessagesCompat,
    GeminiGenerateContentCompat,
}

impl EndpointProtocol {
    pub fn wire_protocol(self) -> ProviderProtocol {
        match self {
            Self::OpenAiResponses | Self::OpenAiResponsesCompat => {
                ProviderProtocol::OpenAiResponses
            }
            Self::OpenAiChatCompletions | Self::OpenAiChatCompletionsCompat => {
                ProviderProtocol::OpenAiChatCompletions
            }
            Self::ClaudeMessages | Self::ClaudeMessagesCompat => ProviderProtocol::ClaudeMessages,
            Self::GeminiGenerateContent | Self::GeminiGenerateContentCompat => {
                ProviderProtocol::GeminiGenerateContent
            }
        }
    }

    pub fn is_compat(self) -> bool {
        matches!(
            self,
            Self::OpenAiResponsesCompat
                | Self::OpenAiChatCompletionsCompat
                | Self::ClaudeMessagesCompat
                | Self::GeminiGenerateContentCompat
        )
    }
}

impl From<ProviderProtocol> for EndpointProtocol {
    fn from(value: ProviderProtocol) -> Self {
        match value {
            ProviderProtocol::OpenAiResponses => Self::OpenAiResponses,
            ProviderProtocol::OpenAiChatCompletions => Self::OpenAiChatCompletions,
            ProviderProtocol::ClaudeMessages => Self::ClaudeMessages,
            ProviderProtocol::GeminiGenerateContent => Self::GeminiGenerateContent,
        }
    }
}

impl FromStr for EndpointProtocol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "responses" | "openai_responses" | "open_ai_responses" => Ok(Self::OpenAiResponses),
            "chat_completions" | "openai_chat_completions" | "open_ai_chat_completions" => {
                Ok(Self::OpenAiChatCompletions)
            }
            "claude_messages" | "anthropic_messages" => Ok(Self::ClaudeMessages),
            "gemini_generate_content" => Ok(Self::GeminiGenerateContent),
            "responses_compat" | "openai_responses_compat" | "open_ai_responses_compat" => {
                Ok(Self::OpenAiResponsesCompat)
            }
            "chat_completions_compat"
            | "openai_chat_completions_compat"
            | "open_ai_chat_completions_compat" => Ok(Self::OpenAiChatCompletionsCompat),
            "claude_messages_compat" | "anthropic_messages_compat" => {
                Ok(Self::ClaudeMessagesCompat)
            }
            "gemini_generate_content_compat" => Ok(Self::GeminiGenerateContentCompat),
            _ => Err(format!(
                "unsupported endpoint protocol `{value}`; expected one of: \
openai_responses, openai_chat_completions, claude_messages, gemini_generate_content, \
openai_responses_compat, openai_chat_completions_compat, claude_messages_compat, \
gemini_generate_content_compat"
            )),
        }
    }
}

/// Authentication strategy for an upstream provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthScheme {
    Bearer,
    Header { name: String },
    Query { name: String },
}

impl AuthScheme {
    pub fn default_for(protocol: EndpointProtocol) -> Self {
        match protocol.wire_protocol() {
            ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => {
                Self::Bearer
            }
            ProviderProtocol::ClaudeMessages => Self::Header {
                name: "x-api-key".into(),
            },
            ProviderProtocol::GeminiGenerateContent => Self::Header {
                name: "x-goog-api-key".into(),
            },
        }
    }
}

/// Target provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpoint {
    pub protocol: EndpointProtocol,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthScheme>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub default_headers: BTreeMap<String, String>,
}

impl ProviderEndpoint {
    pub fn new(protocol: impl Into<EndpointProtocol>, base_url: impl Into<String>) -> Self {
        let protocol = protocol.into();
        let mut endpoint = Self {
            protocol,
            base_url: base_url.into(),
            auth: None,
            default_headers: BTreeMap::new(),
        };

        if matches!(protocol.wire_protocol(), ProviderProtocol::ClaudeMessages) {
            endpoint
                .default_headers
                .insert("anthropic-version".into(), "2023-06-01".into());
        }

        endpoint
    }

    pub fn openai_responses() -> Self {
        Self::new(
            EndpointProtocol::OpenAiResponses,
            "https://api.openai.com/v1",
        )
    }

    pub fn openai_chat_completions() -> Self {
        Self::new(
            EndpointProtocol::OpenAiChatCompletions,
            "https://api.openai.com/v1",
        )
    }

    pub fn claude_messages() -> Self {
        Self::new(
            EndpointProtocol::ClaudeMessages,
            "https://api.anthropic.com/v1",
        )
    }

    pub fn gemini_generate_content() -> Self {
        Self::new(
            EndpointProtocol::GeminiGenerateContent,
            "https://generativelanguage.googleapis.com",
        )
    }

    pub fn openai_responses_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::OpenAiResponsesCompat, base_url)
    }

    pub fn openai_chat_completions_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::OpenAiChatCompletionsCompat, base_url)
    }

    pub fn claude_messages_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::ClaudeMessagesCompat, base_url)
    }

    pub fn gemini_generate_content_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::GeminiGenerateContentCompat, base_url)
    }

    pub fn with_default_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.default_headers.insert(name.into(), value.into());
        self
    }

    pub fn with_auth(mut self, auth: AuthScheme) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn auth_scheme(&self) -> AuthScheme {
        self.auth
            .clone()
            .unwrap_or_else(|| AuthScheme::default_for(self.protocol))
    }

    pub fn wire_protocol(&self) -> ProviderProtocol {
        self.protocol.wire_protocol()
    }

    pub(crate) fn request_url(&self, model: &str, stream: bool) -> String {
        if self.protocol.is_compat() {
            return self.base_url.trim().to_string();
        }

        let base = self.base_url.trim_end_matches('/');
        match self.protocol.wire_protocol() {
            ProviderProtocol::OpenAiResponses => {
                if base.ends_with("/responses") {
                    base.to_string()
                } else {
                    format!("{base}/responses")
                }
            }
            ProviderProtocol::OpenAiChatCompletions => {
                if base.ends_with("/chat/completions") {
                    base.to_string()
                } else {
                    format!("{base}/chat/completions")
                }
            }
            ProviderProtocol::ClaudeMessages => {
                if base.ends_with("/messages") {
                    base.to_string()
                } else {
                    format!("{base}/messages")
                }
            }
            ProviderProtocol::GeminiGenerateContent => {
                let prefix = if base.ends_with("/v1beta") {
                    base.to_string()
                } else {
                    format!("{base}/v1beta")
                };
                if stream {
                    format!("{prefix}/models/{model}:streamGenerateContent?alt=sse")
                } else {
                    format!("{prefix}/models/{model}:generateContent")
                }
            }
        }
    }
}

/// Conversion error raised by the canonical/raw transcoder layer.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid shape: {0}")]
    InvalidShape(String),
    #[error("unsupported feature for target protocol: {0}")]
    UnsupportedFeature(String),
}

/// A raw SSE frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStreamFrame {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub data: String,
}

pub fn parse_request(
    protocol: ProviderProtocol,
    raw_json: &str,
) -> Result<LlmRequest, ProtocolError> {
    let body: Value = serde_json::from_str(raw_json)?;
    match protocol {
        ProviderProtocol::OpenAiResponses => parse_openai_responses_request(&body),
        ProviderProtocol::OpenAiChatCompletions => parse_openai_chat_request(&body),
        ProviderProtocol::ClaudeMessages => parse_claude_request(&body),
        ProviderProtocol::GeminiGenerateContent => parse_gemini_request(&body),
    }
}

pub fn emit_request(
    protocol: ProviderProtocol,
    request: &LlmRequest,
) -> Result<String, ProtocolError> {
    serde_json::to_string(&emit_request_value(protocol, request, false, false)?)
        .map_err(ProtocolError::from)
}

pub fn transcode_request(
    from: ProviderProtocol,
    to: ProviderProtocol,
    raw_json: &str,
) -> Result<String, ProtocolError> {
    let request = parse_request(from, raw_json)?;
    emit_request(to, &request)
}

pub fn parse_response(
    protocol: ProviderProtocol,
    raw_json: &str,
) -> Result<LlmResponse, ProtocolError> {
    let body: Value = serde_json::from_str(raw_json)?;
    match protocol {
        ProviderProtocol::OpenAiResponses => parse_openai_responses_response(&body),
        ProviderProtocol::OpenAiChatCompletions => parse_openai_chat_response(&body),
        ProviderProtocol::ClaudeMessages => parse_claude_response(&body),
        ProviderProtocol::GeminiGenerateContent => parse_gemini_response(&body),
    }
}

pub fn emit_response(
    protocol: ProviderProtocol,
    response: &LlmResponse,
) -> Result<String, ProtocolError> {
    let body = match protocol {
        ProviderProtocol::OpenAiResponses => emit_openai_responses_response(response)?,
        ProviderProtocol::OpenAiChatCompletions => emit_openai_chat_response(response)?,
        ProviderProtocol::ClaudeMessages => emit_claude_response(response)?,
        ProviderProtocol::GeminiGenerateContent => emit_gemini_response(response)?,
    };
    serde_json::to_string(&body).map_err(ProtocolError::from)
}

pub fn transcode_response(
    from: ProviderProtocol,
    to: ProviderProtocol,
    raw_json: &str,
) -> Result<String, ProtocolError> {
    let response = parse_response(from, raw_json)?;
    emit_response(to, &response)
}

pub fn parse_error(
    protocol: ProviderProtocol,
    status: Option<u16>,
    raw_json: &str,
) -> Result<ProviderError, ProtocolError> {
    let body: Value = serde_json::from_str(raw_json)?;
    Ok(match protocol {
        ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => {
            parse_openai_error(protocol, status, &body)
        }
        ProviderProtocol::ClaudeMessages => parse_claude_error(status, &body),
        ProviderProtocol::GeminiGenerateContent => parse_gemini_error(status, &body),
    })
}

pub fn emit_error(
    protocol: ProviderProtocol,
    error: &ProviderError,
) -> Result<String, ProtocolError> {
    let body = match protocol {
        ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => {
            json!({
                "error": {
                    "message": error.message,
                    "type": error.code.clone().unwrap_or_else(|| "invalid_request_error".into()),
                    "code": error.code,
                }
            })
        }
        ProviderProtocol::ClaudeMessages => json!({
            "type": "error",
            "error": {
                "type": error.code.clone().unwrap_or_else(|| "api_error".into()),
                "message": error.message,
            }
        }),
        ProviderProtocol::GeminiGenerateContent => json!({
            "error": {
                "code": error.status.unwrap_or(500),
                "status": error.code.clone().unwrap_or_else(|| "INTERNAL".into()),
                "message": error.message,
            }
        }),
    };
    serde_json::to_string(&body).map_err(ProtocolError::from)
}

pub fn transcode_error(
    from: ProviderProtocol,
    to: ProviderProtocol,
    status: Option<u16>,
    raw_json: &str,
) -> Result<String, ProtocolError> {
    let error = parse_error(from, status, raw_json)?;
    emit_error(to, &error)
}

pub fn parse_stream_event(
    protocol: ProviderProtocol,
    frame: &ProviderStreamFrame,
) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    if frame.data.trim() == "[DONE]" {
        return Ok(None);
    }

    let body: Value = serde_json::from_str(&frame.data)?;
    match protocol {
        ProviderProtocol::OpenAiResponses => parse_openai_responses_stream_event(frame, &body),
        ProviderProtocol::OpenAiChatCompletions => parse_openai_chat_stream_event(&body),
        ProviderProtocol::ClaudeMessages => parse_claude_stream_event(frame, &body),
        ProviderProtocol::GeminiGenerateContent => parse_gemini_stream_event(&body),
    }
}

pub fn emit_stream_event(
    protocol: ProviderProtocol,
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let frame = match protocol {
        ProviderProtocol::OpenAiResponses => emit_openai_responses_stream_event(event)?,
        ProviderProtocol::OpenAiChatCompletions => emit_openai_chat_stream_event(event)?,
        ProviderProtocol::ClaudeMessages => emit_claude_stream_event(event)?,
        ProviderProtocol::GeminiGenerateContent => emit_gemini_stream_event(event)?,
    };
    Ok(frame)
}

pub fn transcode_stream_event(
    from: ProviderProtocol,
    to: ProviderProtocol,
    frame: &ProviderStreamFrame,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    match parse_stream_event(from, frame)? {
        Some(event) => emit_stream_event(to, &event),
        None => Ok(None),
    }
}

pub(crate) fn emit_request_with_mode(
    protocol: ProviderProtocol,
    request: &LlmRequest,
    stream: bool,
) -> Result<String, ProtocolError> {
    serde_json::to_string(&emit_request_value(protocol, request, stream, true)?)
        .map_err(ProtocolError::from)
}

pub(crate) fn take_sse_frames(buffer: &mut String) -> Vec<ProviderStreamFrame> {
    let normalized = buffer.replace("\r\n", "\n");
    *buffer = normalized;

    let mut frames = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let block = buffer[..idx].to_string();
        *buffer = buffer[idx + 2..].to_string();

        let mut event = None;
        let mut data_lines = Vec::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("event:") {
                event = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim_start().to_string());
            }
        }

        if !data_lines.is_empty() {
            frames.push(ProviderStreamFrame {
                event,
                data: data_lines.join("\n"),
            });
        }
    }

    frames
}

fn parse_openai_responses_request(body: &Value) -> Result<LlmRequest, ProtocolError> {
    let model = required_str(body, "model")?.to_string();
    let instructions = body
        .get("instructions")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let input = parse_openai_responses_input(body.get("input").unwrap_or(&Value::Null))?;
    let messages = input
        .iter()
        .filter_map(RequestItem::as_message)
        .cloned()
        .collect::<Vec<_>>();
    let capabilities = parse_openai_responses_capabilities(body)?;
    let generation = parse_generation(
        body.get("max_output_tokens").and_then(Value::as_u64),
        body.get("temperature").and_then(Value::as_f64),
        body.get("top_p").and_then(Value::as_f64),
        body.get("top_k").and_then(Value::as_u64),
        string_or_array(body.get("stop")),
        None,
        None,
        body.get("seed").and_then(Value::as_u64),
    );

    Ok(LlmRequest {
        model,
        instructions,
        input,
        messages,
        capabilities,
        generation,
        metadata: object_to_extensions(body.get("metadata")),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_request_value(
    protocol: ProviderProtocol,
    request: &LlmRequest,
    stream: bool,
    transport: bool,
) -> Result<Value, ProtocolError> {
    match protocol {
        ProviderProtocol::OpenAiResponses => emit_openai_responses_request(request, stream),
        ProviderProtocol::OpenAiChatCompletions => emit_openai_chat_request(request, stream),
        ProviderProtocol::ClaudeMessages => emit_claude_request(request, stream),
        ProviderProtocol::GeminiGenerateContent => {
            if transport {
                emit_gemini_transport_request(request)
            } else {
                emit_gemini_request(request)
            }
        }
    }
}

fn emit_openai_responses_request(
    request: &LlmRequest,
    stream: bool,
) -> Result<Value, ProtocolError> {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));

    if let Some(instructions) = request.normalized_instructions() {
        map.insert("instructions".into(), Value::String(instructions));
    }

    let input = request_items_for_instructionless_protocol(request)
        .into_iter()
        .map(openai_responses_input_item)
        .collect::<Result<Vec<_>, _>>()?;
    if !input.is_empty() {
        map.insert("input".into(), Value::Array(input));
    }

    emit_generation_common(&mut map, &request.generation, true);
    emit_openai_responses_capabilities(&mut map, &request.capabilities)?;

    if !request.metadata.is_empty() {
        map.insert(
            "metadata".into(),
            Value::Object(extensions_to_object(&request.metadata)),
        );
    }
    if stream {
        map.insert("stream".into(), Value::Bool(true));
    }

    Ok(Value::Object(map))
}

fn parse_openai_chat_request(body: &Value) -> Result<LlmRequest, ProtocolError> {
    let model = required_str(body, "model")?.to_string();
    let messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("messages".into()))?
        .iter()
        .map(parse_openai_chat_message)
        .collect::<Result<Vec<_>, _>>()?;
    let input = messages.iter().cloned().map(RequestItem::from).collect();
    let capabilities = CapabilitySet {
        tools: parse_function_tools(body.get("tools"))?,
        structured_output: parse_openai_chat_structured_output(body),
        reasoning: None,
        modalities: vec![OutputModality::Text],
        safety: None,
        cache: None,
        builtin_tools: Vec::new(),
        vendor_extensions: VendorExtensions::new(),
    };
    let generation = parse_generation(
        body.get("max_tokens").and_then(Value::as_u64),
        body.get("temperature").and_then(Value::as_f64),
        body.get("top_p").and_then(Value::as_f64),
        None,
        string_or_array(body.get("stop")),
        body.get("presence_penalty").and_then(Value::as_f64),
        body.get("frequency_penalty").and_then(Value::as_f64),
        body.get("seed").and_then(Value::as_u64),
    );

    Ok(LlmRequest {
        model,
        instructions: None,
        input,
        messages,
        capabilities,
        generation,
        metadata: VendorExtensions::new(),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_openai_chat_request(request: &LlmRequest, stream: bool) -> Result<Value, ProtocolError> {
    if !request.capabilities.builtin_tools.is_empty() {
        return Err(ProtocolError::UnsupportedFeature(
            "builtin tools in OpenAI Chat Completions".into(),
        ));
    }
    if request.capabilities.reasoning.is_some() {
        return Err(ProtocolError::UnsupportedFeature(
            "reasoning capability in OpenAI Chat Completions".into(),
        ));
    }

    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));

    let messages = chat_messages_with_instructions(request)
        .into_iter()
        .map(openai_chat_message_json)
        .collect::<Result<Vec<_>, _>>()?;
    map.insert("messages".into(), Value::Array(messages));

    let tools = emit_function_tools(&request.capabilities.tools);
    if !tools.is_empty() {
        map.insert("tools".into(), Value::Array(tools));
    }

    if let Some(structured_output) = &request.capabilities.structured_output {
        map.insert(
            "response_format".into(),
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": structured_output
                        .name
                        .clone()
                        .unwrap_or_else(|| "response".into()),
                    "schema": structured_output.schema,
                    "strict": structured_output.strict,
                }
            }),
        );
    }

    emit_generation_common(&mut map, &request.generation, false);

    if stream {
        map.insert("stream".into(), Value::Bool(true));
    }

    Ok(Value::Object(map))
}

fn parse_claude_request(body: &Value) -> Result<LlmRequest, ProtocolError> {
    let model = required_str(body, "model")?.to_string();
    let system = match body.get("system") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Array(blocks)) => {
            let joined = blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        _ => None,
    };
    let mut messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("messages".into()))?
        .iter()
        .map(parse_claude_message)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(text) = &system {
        messages.insert(0, Message::text(MessageRole::System, text.clone()));
    }
    let input = messages.iter().cloned().map(RequestItem::from).collect();
    let capabilities = CapabilitySet {
        tools: parse_claude_tools(body.get("tools"))?,
        structured_output: None,
        reasoning: None,
        modalities: vec![OutputModality::Text],
        safety: None,
        cache: None,
        builtin_tools: Vec::new(),
        vendor_extensions: VendorExtensions::new(),
    };
    let generation = parse_generation(
        body.get("max_tokens").and_then(Value::as_u64),
        body.get("temperature").and_then(Value::as_f64),
        body.get("top_p").and_then(Value::as_f64),
        None,
        string_or_array(body.get("stop_sequences")),
        None,
        None,
        None,
    );

    Ok(LlmRequest {
        model,
        instructions: system,
        input,
        messages,
        capabilities,
        generation,
        metadata: VendorExtensions::new(),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_claude_request(request: &LlmRequest, stream: bool) -> Result<Value, ProtocolError> {
    if !request.capabilities.builtin_tools.is_empty() {
        return Err(ProtocolError::UnsupportedFeature(
            "builtin tools in Claude Messages".into(),
        ));
    }
    if request.capabilities.structured_output.is_some() {
        return Err(ProtocolError::UnsupportedFeature(
            "structured output in Claude Messages".into(),
        ));
    }
    if request.capabilities.reasoning.is_some() {
        return Err(ProtocolError::UnsupportedFeature(
            "reasoning capability in Claude Messages".into(),
        ));
    }

    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    map.insert(
        "max_tokens".into(),
        Value::from(request.generation.max_output_tokens.unwrap_or(1024)),
    );

    if let Some(system) = request.normalized_instructions() {
        map.insert("system".into(), Value::String(system));
    }

    let messages = request_messages_for_separate_instruction_protocol(request)
        .into_iter()
        .map(claude_message_json)
        .collect::<Result<Vec<_>, _>>()?;
    map.insert("messages".into(), Value::Array(messages));

    let tools = emit_claude_tools(&request.capabilities.tools);
    if !tools.is_empty() {
        map.insert("tools".into(), Value::Array(tools));
    }

    if let Some(temperature) = request.generation.temperature {
        map.insert("temperature".into(), Value::from(temperature));
    }
    if let Some(top_p) = request.generation.top_p {
        map.insert("top_p".into(), Value::from(top_p));
    }
    if !request.generation.stop_sequences.is_empty() {
        map.insert(
            "stop_sequences".into(),
            Value::Array(
                request
                    .generation
                    .stop_sequences
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if stream {
        map.insert("stream".into(), Value::Bool(true));
    }

    Ok(Value::Object(map))
}

fn parse_gemini_request(body: &Value) -> Result<LlmRequest, ProtocolError> {
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError::MissingField("model".into()))?
        .to_string();

    let instructions = body
        .get("systemInstruction")
        .map(parse_gemini_instruction)
        .transpose()?;
    let mut messages = body
        .get("contents")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("contents".into()))?
        .iter()
        .map(parse_gemini_content)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(text) = &instructions {
        messages.insert(0, Message::text(MessageRole::System, text.clone()));
    }
    let input = messages.iter().cloned().map(RequestItem::from).collect();
    let capabilities = parse_gemini_capabilities(body)?;
    let generation_config = body.get("generationConfig");
    let generation = parse_generation(
        generation_config
            .and_then(|value| value.get("maxOutputTokens"))
            .and_then(Value::as_u64),
        generation_config
            .and_then(|value| value.get("temperature"))
            .and_then(Value::as_f64),
        generation_config
            .and_then(|value| value.get("topP"))
            .and_then(Value::as_f64),
        generation_config
            .and_then(|value| value.get("topK"))
            .and_then(Value::as_u64),
        generation_config
            .and_then(|value| value.get("stopSequences"))
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        None,
        None,
        generation_config
            .and_then(|value| value.get("seed"))
            .and_then(Value::as_u64),
    );

    Ok(LlmRequest {
        model,
        instructions,
        input,
        messages,
        capabilities,
        generation,
        metadata: VendorExtensions::new(),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_gemini_request(request: &LlmRequest) -> Result<Value, ProtocolError> {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    emit_gemini_request_inner(request, map)
}

fn emit_gemini_transport_request(request: &LlmRequest) -> Result<Value, ProtocolError> {
    emit_gemini_request_inner(request, Map::new())
}

fn emit_gemini_request_inner(
    request: &LlmRequest,
    mut map: Map<String, Value>,
) -> Result<Value, ProtocolError> {
    let contents = request_messages_for_separate_instruction_protocol(request)
        .into_iter()
        .map(gemini_content_json)
        .collect::<Result<Vec<_>, _>>()?;
    map.insert("contents".into(), Value::Array(contents));

    if let Some(instructions) = request.normalized_instructions() {
        map.insert(
            "systemInstruction".into(),
            json!({
                "role": "system",
                "parts": [{ "text": instructions }],
            }),
        );
    }

    let tools = emit_gemini_tools(&request.capabilities)?;
    if !tools.is_empty() {
        map.insert("tools".into(), Value::Array(tools));
    }

    let mut generation_config = Map::new();
    if let Some(max_tokens) = request.generation.max_output_tokens {
        generation_config.insert("maxOutputTokens".into(), Value::from(max_tokens));
    }
    if let Some(temperature) = request.generation.temperature {
        generation_config.insert("temperature".into(), Value::from(temperature));
    }
    if let Some(top_p) = request.generation.top_p {
        generation_config.insert("topP".into(), Value::from(top_p));
    }
    if let Some(top_k) = request.generation.top_k {
        generation_config.insert("topK".into(), Value::from(top_k));
    }
    if !request.generation.stop_sequences.is_empty() {
        generation_config.insert(
            "stopSequences".into(),
            Value::Array(
                request
                    .generation
                    .stop_sequences
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if let Some(seed) = request.generation.seed {
        generation_config.insert("seed".into(), Value::from(seed));
    }
    if let Some(structured_output) = &request.capabilities.structured_output {
        generation_config.insert(
            "responseMimeType".into(),
            Value::String("application/json".into()),
        );
        generation_config.insert("responseSchema".into(), structured_output.schema.clone());
    }
    if !generation_config.is_empty() {
        map.insert("generationConfig".into(), Value::Object(generation_config));
    }

    Ok(Value::Object(map))
}

fn parse_openai_responses_response(body: &Value) -> Result<LlmResponse, ProtocolError> {
    let output: Vec<ResponseItem> = body
        .get("output")
        .and_then(Value::as_array)
        .map(|items| parse_openai_responses_output(items))
        .transpose()?
        .unwrap_or_default();
    let messages = output
        .iter()
        .filter_map(response_item_as_message)
        .cloned()
        .collect::<Vec<_>>();
    let content_text = body
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| collect_message_text(&messages));
    let usage = TokenUsage {
        prompt_tokens: body
            .get("usage")
            .and_then(|value| value.get("input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        completion_tokens: body
            .get("usage")
            .and_then(|value| value.get("output_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        total_tokens: body
            .get("usage")
            .and_then(|value| value.get("total_tokens"))
            .and_then(Value::as_u64)
            .map(|value| value as u32),
    };

    Ok(LlmResponse {
        output,
        messages,
        content_text,
        usage,
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::OpenAiResponses,
        finish_reason: body.get("status").and_then(parse_finish_reason),
        response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_openai_responses_response(response: &LlmResponse) -> Result<Value, ProtocolError> {
    let output = response_output_items(response)
        .into_iter()
        .map(openai_responses_output_item)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "id": response.response_id,
        "model": response.model,
        "status": response.finish_reason.as_ref().map(finish_reason_string),
        "output_text": response.content_text,
        "output": output,
        "usage": {
            "input_tokens": response.usage.prompt_tokens,
            "output_tokens": response.usage.completion_tokens,
            "total_tokens": response.usage.total(),
        }
    }))
}

fn parse_openai_chat_response(body: &Value) -> Result<LlmResponse, ProtocolError> {
    let choice = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| ProtocolError::MissingField("choices[0]".into()))?;
    let message = parse_openai_chat_message(choice.get("message").unwrap_or(&Value::Null))?;
    let mut output = response_items_from_message(&message);
    if output.is_empty() {
        output.push(ResponseItem::Message {
            message: message.clone(),
        });
    }
    Ok(LlmResponse {
        output,
        messages: vec![message.clone()],
        content_text: message.plain_text(),
        usage: TokenUsage {
            prompt_tokens: body
                .get("usage")
                .and_then(|value| value.get("prompt_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            completion_tokens: body
                .get("usage")
                .and_then(|value| value.get("completion_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            total_tokens: body
                .get("usage")
                .and_then(|value| value.get("total_tokens"))
                .and_then(Value::as_u64)
                .map(|value| value as u32),
        },
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::OpenAiChatCompletions,
        finish_reason: choice.get("finish_reason").and_then(parse_finish_reason),
        response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_openai_chat_response(response: &LlmResponse) -> Result<Value, ProtocolError> {
    let message = assistant_message_from_response(response)
        .unwrap_or_else(|| Message::text(MessageRole::Assistant, response.content_text.clone()));
    Ok(json!({
        "id": response.response_id,
        "model": response.model,
        "choices": [{
            "index": 0,
            "finish_reason": response.finish_reason.as_ref().map(finish_reason_string),
            "message": openai_chat_message_json(message)?,
        }],
        "usage": {
            "prompt_tokens": response.usage.prompt_tokens,
            "completion_tokens": response.usage.completion_tokens,
            "total_tokens": response.usage.total(),
        }
    }))
}

fn parse_claude_response(body: &Value) -> Result<LlmResponse, ProtocolError> {
    let message = parse_claude_message(body)?;
    let output = response_items_from_message(&message);
    Ok(LlmResponse {
        output,
        messages: vec![message.clone()],
        content_text: message.plain_text(),
        usage: TokenUsage {
            prompt_tokens: body
                .get("usage")
                .and_then(|value| value.get("input_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            completion_tokens: body
                .get("usage")
                .and_then(|value| value.get("output_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            total_tokens: None,
        },
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::ClaudeMessages,
        finish_reason: body.get("stop_reason").and_then(parse_finish_reason),
        response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_claude_response(response: &LlmResponse) -> Result<Value, ProtocolError> {
    let message = assistant_message_from_response(response)
        .unwrap_or_else(|| Message::text(MessageRole::Assistant, response.content_text.clone()));
    Ok(json!({
        "id": response.response_id,
        "type": "message",
        "role": "assistant",
        "model": response.model,
        "stop_reason": response.finish_reason.as_ref().map(finish_reason_string),
        "content": claude_content_parts(&message.parts)?,
        "usage": {
            "input_tokens": response.usage.prompt_tokens,
            "output_tokens": response.usage.completion_tokens,
        }
    }))
}

fn parse_gemini_response(body: &Value) -> Result<LlmResponse, ProtocolError> {
    let candidate = body
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .ok_or_else(|| ProtocolError::MissingField("candidates[0]".into()))?;
    let message = parse_gemini_candidate(candidate)?;
    let output = response_items_from_message(&message);
    Ok(LlmResponse {
        output,
        messages: vec![message.clone()],
        content_text: message.plain_text(),
        usage: TokenUsage {
            prompt_tokens: body
                .get("usageMetadata")
                .and_then(|value| value.get("promptTokenCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            completion_tokens: body
                .get("usageMetadata")
                .and_then(|value| value.get("candidatesTokenCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            total_tokens: body
                .get("usageMetadata")
                .and_then(|value| value.get("totalTokenCount"))
                .and_then(Value::as_u64)
                .map(|value| value as u32),
        },
        model: body
            .get("modelVersion")
            .or_else(|| body.get("model"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::GeminiGenerateContent,
        finish_reason: candidate.get("finishReason").and_then(parse_finish_reason),
        response_id: body
            .get("responseId")
            .and_then(Value::as_str)
            .map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn emit_gemini_response(response: &LlmResponse) -> Result<Value, ProtocolError> {
    let message = assistant_message_from_response(response)
        .unwrap_or_else(|| Message::text(MessageRole::Assistant, response.content_text.clone()));
    Ok(json!({
        "modelVersion": response.model,
        "responseId": response.response_id,
        "candidates": [{
            "finishReason": response.finish_reason.as_ref().map(finish_reason_string_upper_camel),
            "content": gemini_content_json(message)?,
        }],
        "usageMetadata": {
            "promptTokenCount": response.usage.prompt_tokens,
            "candidatesTokenCount": response.usage.completion_tokens,
            "totalTokenCount": response.usage.total(),
        }
    }))
}

fn parse_openai_responses_stream_event(
    frame: &ProviderStreamFrame,
    body: &Value,
) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    let event = frame
        .event
        .clone()
        .or_else(|| body.get("type").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default();

    Ok(match event.as_str() {
        "response.created" | "response.in_progress" => Some(LlmStreamEvent::ResponseStarted {
            response_id: body
                .get("response")
                .and_then(|v| v.get("id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| body.get("id").and_then(Value::as_str).map(str::to_owned)),
            model: body
                .get("response")
                .and_then(|v| v.get("model"))
                .or_else(|| body.get("model"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            provider_protocol: ProviderProtocol::OpenAiResponses,
        }),
        "response.output_text.delta" => Some(LlmStreamEvent::TextDelta {
            delta: body
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }),
        "response.function_call_arguments.delta" => Some(LlmStreamEvent::ToolCallDelta {
            call_id: body
                .get("item_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            name: body
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            delta: body
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }),
        "response.output_item.added" => body
            .get("item")
            .map(parse_openai_responses_single_output_item)
            .transpose()?
            .map(|item| LlmStreamEvent::OutputItemAdded { item }),
        "response.reasoning_summary_text.delta" => Some(LlmStreamEvent::ReasoningDelta {
            delta: body
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }),
        "response.completed" => {
            let response = body.get("response").unwrap_or(body);
            Some(LlmStreamEvent::Completed {
                response: parse_openai_responses_response(response)?,
            })
        }
        "response.error" | "error" => Some(LlmStreamEvent::Error {
            message: body
                .get("message")
                .or_else(|| body.get("error").and_then(|value| value.get("message")))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        }),
        _ => None,
    })
}

fn emit_openai_responses_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let frame = match event {
        LlmStreamEvent::ResponseStarted {
            response_id, model, ..
        } => ProviderStreamFrame {
            event: Some("response.created".into()),
            data: serde_json::to_string(&json!({
                "type": "response.created",
                "response": { "id": response_id, "model": model }
            }))?,
        },
        LlmStreamEvent::TextDelta { delta } => ProviderStreamFrame {
            event: Some("response.output_text.delta".into()),
            data: serde_json::to_string(&json!({
                "type": "response.output_text.delta",
                "delta": delta
            }))?,
        },
        LlmStreamEvent::ToolCallDelta {
            call_id,
            name,
            delta,
        } => ProviderStreamFrame {
            event: Some("response.function_call_arguments.delta".into()),
            data: serde_json::to_string(&json!({
                "type": "response.function_call_arguments.delta",
                "item_id": call_id,
                "name": name,
                "delta": delta
            }))?,
        },
        LlmStreamEvent::OutputItemAdded { item } => ProviderStreamFrame {
            event: Some("response.output_item.added".into()),
            data: serde_json::to_string(&json!({
                "type": "response.output_item.added",
                "item": openai_responses_output_item(item.clone())?
            }))?,
        },
        LlmStreamEvent::ReasoningDelta { delta } => ProviderStreamFrame {
            event: Some("response.reasoning_summary_text.delta".into()),
            data: serde_json::to_string(&json!({
                "type": "response.reasoning_summary_text.delta",
                "delta": delta
            }))?,
        },
        LlmStreamEvent::Completed { response } => ProviderStreamFrame {
            event: Some("response.completed".into()),
            data: serde_json::to_string(&json!({
                "type": "response.completed",
                "response": emit_openai_responses_response(response)?
            }))?,
        },
        LlmStreamEvent::Error { message } => ProviderStreamFrame {
            event: Some("response.error".into()),
            data: serde_json::to_string(&json!({ "type": "response.error", "message": message }))?,
        },
        LlmStreamEvent::Usage { .. }
        | LlmStreamEvent::ContentPartAdded { .. }
        | LlmStreamEvent::ToolResult { .. } => return Ok(None),
    };
    Ok(Some(frame))
}

fn parse_openai_chat_stream_event(body: &Value) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    let choice = match body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
    {
        Some(choice) => choice,
        None => return Ok(None),
    };

    if let Some(role) = choice
        .get("delta")
        .and_then(|value| value.get("role"))
        .and_then(Value::as_str)
    {
        return Ok(Some(LlmStreamEvent::ResponseStarted {
            response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
            model: body
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or(role)
                .to_string(),
            provider_protocol: ProviderProtocol::OpenAiChatCompletions,
        }));
    }

    if let Some(delta) = choice
        .get("delta")
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
    {
        return Ok(Some(LlmStreamEvent::TextDelta {
            delta: delta.to_string(),
        }));
    }

    if let Some(tool_calls) = choice
        .get("delta")
        .and_then(|value| value.get("tool_calls"))
        .and_then(Value::as_array)
    {
        if let Some(tool_call) = tool_calls.first() {
            return Ok(Some(LlmStreamEvent::ToolCallDelta {
                call_id: tool_call
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                name: tool_call
                    .get("function")
                    .and_then(|value| value.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                delta: tool_call
                    .get("function")
                    .and_then(|value| value.get("arguments"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            }));
        }
    }

    if let Some(usage) = body.get("usage") {
        return Ok(Some(LlmStreamEvent::Usage {
            usage: TokenUsage {
                prompt_tokens: usage
                    .get("prompt_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                completion_tokens: usage
                    .get("completion_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                total_tokens: usage
                    .get("total_tokens")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
            },
        }));
    }

    Ok(None)
}

fn emit_openai_chat_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let data = match event {
        LlmStreamEvent::ResponseStarted {
            response_id, model, ..
        } => json!({
            "id": response_id,
            "model": model,
            "choices": [{ "index": 0, "delta": { "role": "assistant" } }]
        }),
        LlmStreamEvent::TextDelta { delta } => json!({
            "choices": [{ "index": 0, "delta": { "content": delta } }]
        }),
        LlmStreamEvent::ToolCallDelta {
            call_id,
            name,
            delta,
        } => json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "id": call_id,
                        "type": "function",
                        "function": { "name": name, "arguments": delta }
                    }]
                }
            }]
        }),
        LlmStreamEvent::Usage { usage } => json!({
            "usage": {
                "prompt_tokens": usage.prompt_tokens,
                "completion_tokens": usage.completion_tokens,
                "total_tokens": usage.total()
            }
        }),
        LlmStreamEvent::Completed { .. } => {
            return Ok(Some(ProviderStreamFrame {
                event: None,
                data: "[DONE]".into(),
            }))
        }
        _ => return Ok(None),
    };

    Ok(Some(ProviderStreamFrame {
        event: None,
        data: serde_json::to_string(&data)?,
    }))
}

fn parse_claude_stream_event(
    frame: &ProviderStreamFrame,
    body: &Value,
) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    match frame.event.as_deref().unwrap_or_default() {
        "message_start" => Ok(Some(LlmStreamEvent::ResponseStarted {
            response_id: body
                .get("message")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            model: body
                .get("message")
                .and_then(|value| value.get("model"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            provider_protocol: ProviderProtocol::ClaudeMessages,
        })),
        "content_block_delta" => {
            let delta = body
                .get("delta")
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if delta.is_empty() {
                Ok(None)
            } else {
                Ok(Some(LlmStreamEvent::TextDelta { delta }))
            }
        }
        "message_delta" => {
            let usage = body.get("usage").map(|usage| TokenUsage {
                prompt_tokens: usage
                    .get("input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                completion_tokens: usage
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                total_tokens: None,
            });
            Ok(usage.map(|usage| LlmStreamEvent::Usage { usage }))
        }
        "message_stop" => Ok(None),
        "error" => Ok(Some(LlmStreamEvent::Error {
            message: body
                .get("error")
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        })),
        _ => Ok(None),
    }
}

fn emit_claude_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let frame = match event {
        LlmStreamEvent::ResponseStarted {
            response_id, model, ..
        } => ProviderStreamFrame {
            event: Some("message_start".into()),
            data: serde_json::to_string(&json!({
                "message": { "id": response_id, "model": model }
            }))?,
        },
        LlmStreamEvent::TextDelta { delta } => ProviderStreamFrame {
            event: Some("content_block_delta".into()),
            data: serde_json::to_string(&json!({
                "delta": { "type": "text_delta", "text": delta }
            }))?,
        },
        LlmStreamEvent::Usage { usage } => ProviderStreamFrame {
            event: Some("message_delta".into()),
            data: serde_json::to_string(&json!({
                "usage": {
                    "input_tokens": usage.prompt_tokens,
                    "output_tokens": usage.completion_tokens
                }
            }))?,
        },
        LlmStreamEvent::Completed { .. } => ProviderStreamFrame {
            event: Some("message_stop".into()),
            data: "{}".into(),
        },
        LlmStreamEvent::Error { message } => ProviderStreamFrame {
            event: Some("error".into()),
            data: serde_json::to_string(&json!({
                "error": { "message": message }
            }))?,
        },
        _ => return Ok(None),
    };
    Ok(Some(frame))
}

fn parse_gemini_stream_event(body: &Value) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    if let Some(candidate) = body
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
    {
        let message = parse_gemini_candidate(candidate)?;
        let text = message.plain_text();
        if !text.is_empty() {
            return Ok(Some(LlmStreamEvent::TextDelta { delta: text }));
        }
    }

    if let Some(usage) = body.get("usageMetadata") {
        return Ok(Some(LlmStreamEvent::Usage {
            usage: TokenUsage {
                prompt_tokens: usage
                    .get("promptTokenCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                completion_tokens: usage
                    .get("candidatesTokenCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                total_tokens: usage
                    .get("totalTokenCount")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
            },
        }));
    }

    Ok(None)
}

fn emit_gemini_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let data = match event {
        LlmStreamEvent::TextDelta { delta } => json!({
            "candidates": [{
                "content": { "role": "model", "parts": [{ "text": delta }] }
            }]
        }),
        LlmStreamEvent::Usage { usage } => json!({
            "usageMetadata": {
                "promptTokenCount": usage.prompt_tokens,
                "candidatesTokenCount": usage.completion_tokens,
                "totalTokenCount": usage.total(),
            }
        }),
        LlmStreamEvent::Completed { response } => emit_gemini_response(response)?,
        _ => return Ok(None),
    };
    Ok(Some(ProviderStreamFrame {
        event: None,
        data: serde_json::to_string(&data)?,
    }))
}

fn parse_openai_error(
    protocol: ProviderProtocol,
    status: Option<u16>,
    body: &Value,
) -> ProviderError {
    ProviderError {
        protocol,
        status,
        code: body
            .get("error")
            .and_then(|value| value.get("code"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                body.get("error")
                    .and_then(|value| value.get("type"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            }),
        message: body
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("provider error")
            .to_string(),
        retry_after: None,
        raw_body: Some(body.to_string()),
        vendor_extensions: VendorExtensions::new(),
    }
}

fn parse_claude_error(status: Option<u16>, body: &Value) -> ProviderError {
    ProviderError {
        protocol: ProviderProtocol::ClaudeMessages,
        status,
        code: body
            .get("error")
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        message: body
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("provider error")
            .to_string(),
        retry_after: None,
        raw_body: Some(body.to_string()),
        vendor_extensions: VendorExtensions::new(),
    }
}

fn parse_gemini_error(status: Option<u16>, body: &Value) -> ProviderError {
    ProviderError {
        protocol: ProviderProtocol::GeminiGenerateContent,
        status: status.or_else(|| {
            body.get("error")
                .and_then(|value| value.get("code"))
                .and_then(Value::as_u64)
                .map(|v| v as u16)
        }),
        code: body
            .get("error")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        message: body
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("provider error")
            .to_string(),
        retry_after: None,
        raw_body: Some(body.to_string()),
        vendor_extensions: VendorExtensions::new(),
    }
}

fn parse_openai_responses_capabilities(body: &Value) -> Result<CapabilitySet, ProtocolError> {
    let mut capabilities = CapabilitySet::default();
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        for tool in tools {
            let tool_type = tool
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("function");
            if tool_type == "function" {
                let function = tool.get("function").unwrap_or(tool);
                capabilities.tools.push(ToolDefinition {
                    name: required_str(function, "name")?.to_string(),
                    description: function
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    input_schema: function
                        .get("parameters")
                        .cloned()
                        .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                    strict: function
                        .get("strict")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    vendor_extensions: VendorExtensions::new(),
                });
            } else {
                capabilities
                    .builtin_tools
                    .push(parse_builtin_tool(tool_type, tool));
            }
        }
    }

    capabilities.reasoning = body.get("reasoning").map(|reasoning| ReasoningCapability {
        effort: reasoning
            .get("effort")
            .and_then(Value::as_str)
            .map(str::to_owned),
        summary: reasoning
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    });

    capabilities.structured_output = body
        .get("text")
        .and_then(|value| value.get("format"))
        .and_then(parse_json_schema_format);

    Ok(capabilities)
}

fn emit_openai_responses_capabilities(
    map: &mut Map<String, Value>,
    capabilities: &CapabilitySet,
) -> Result<(), ProtocolError> {
    let mut tools = emit_openai_responses_function_tools(&capabilities.tools);
    for builtin in &capabilities.builtin_tools {
        tools.push(openai_builtin_tool_json(builtin.clone())?);
    }
    if !tools.is_empty() {
        map.insert("tools".into(), Value::Array(tools));
    }
    if let Some(structured_output) = &capabilities.structured_output {
        map.insert(
            "text".into(),
            json!({
                "format": {
                    "type": "json_schema",
                    "name": structured_output
                        .name
                        .clone()
                        .unwrap_or_else(|| "response".into()),
                    "schema": structured_output.schema,
                    "strict": structured_output.strict,
                }
            }),
        );
    }
    if let Some(reasoning) = &capabilities.reasoning {
        let mut reasoning_map = Map::new();
        if let Some(effort) = &reasoning.effort {
            reasoning_map.insert("effort".into(), Value::String(effort.clone()));
        }
        if let Some(summary) = &reasoning.summary {
            reasoning_map.insert("summary".into(), Value::String(summary.clone()));
        }
        map.insert("reasoning".into(), Value::Object(reasoning_map));
    }
    Ok(())
}

fn parse_openai_responses_input(input: &Value) -> Result<Vec<RequestItem>, ProtocolError> {
    match input {
        Value::Null => Ok(Vec::new()),
        Value::String(text) => Ok(vec![RequestItem::from(Message::text(
            MessageRole::User,
            text.clone(),
        ))]),
        Value::Array(items) => items
            .iter()
            .map(parse_openai_responses_input_item)
            .collect(),
        _ => Err(ProtocolError::InvalidShape("input".into())),
    }
}

fn parse_openai_responses_input_item(item: &Value) -> Result<RequestItem, ProtocolError> {
    if let Some(role) = item.get("role").and_then(Value::as_str) {
        let role = parse_message_role(role);
        let parts =
            parse_openai_responses_content_parts(item.get("content").unwrap_or(&Value::Null))?;
        return Ok(RequestItem::from(Message {
            role,
            parts,
            raw_message: Some(item.to_string()),
            vendor_extensions: VendorExtensions::new(),
        }));
    }

    match item.get("type").and_then(Value::as_str).unwrap_or_default() {
        "function_call_output" => Ok(RequestItem::ToolResult {
            result: ToolResultPart {
                call_id: required_str(item, "call_id")?.to_string(),
                name: item.get("name").and_then(Value::as_str).map(str::to_owned),
                output: item.get("output").cloned().unwrap_or(Value::Null),
                is_error: item
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            },
        }),
        _ => Err(ProtocolError::InvalidShape(
            "unsupported OpenAI Responses input item".into(),
        )),
    }
}

fn openai_responses_input_item(item: RequestItem) -> Result<Value, ProtocolError> {
    match item {
        RequestItem::Message { message } => Ok(json!({
            "role": message_role_string(message.role),
            "content": openai_responses_content_parts(&message.parts)?,
        })),
        RequestItem::ToolResult { result } => Ok(json!({
            "type": "function_call_output",
            "call_id": result.call_id,
            "name": result.name,
            "output": result.output,
            "is_error": result.is_error,
        })),
    }
}

fn parse_openai_responses_output(items: &[Value]) -> Result<Vec<ResponseItem>, ProtocolError> {
    items
        .iter()
        .map(parse_openai_responses_single_output_item)
        .collect()
}

fn parse_openai_responses_single_output_item(item: &Value) -> Result<ResponseItem, ProtocolError> {
    Ok(
        match item.get("type").and_then(Value::as_str).unwrap_or_default() {
            "message" => ResponseItem::Message {
                message: Message {
                    role: parse_message_role(
                        item.get("role")
                            .and_then(Value::as_str)
                            .unwrap_or("assistant"),
                    ),
                    parts: parse_openai_responses_content_parts(
                        item.get("content").unwrap_or(&Value::Null),
                    )?,
                    raw_message: Some(item.to_string()),
                    vendor_extensions: VendorExtensions::new(),
                },
            },
            "function_call" => ResponseItem::ToolCall {
                call: ToolCallPart {
                    call_id: item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    arguments: parse_maybe_json(
                        item.get("arguments")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(Map::new())),
                    ),
                },
            },
            "function_call_output" => ResponseItem::ToolResult {
                result: ToolResultPart {
                    call_id: item
                        .get("call_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: item.get("name").and_then(Value::as_str).map(str::to_owned),
                    output: item.get("output").cloned().unwrap_or(Value::Null),
                    is_error: item
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                },
            },
            "reasoning" => ResponseItem::Reasoning {
                text: item
                    .get("summary")
                    .or_else(|| item.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            "refusal" => ResponseItem::Refusal {
                text: item
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            other => {
                return Err(ProtocolError::InvalidShape(format!(
                    "unsupported OpenAI Responses output item type: {other}"
                )))
            }
        },
    )
}

fn openai_responses_output_item(item: ResponseItem) -> Result<Value, ProtocolError> {
    match item {
        ResponseItem::Message { message } => Ok(json!({
            "type": "message",
            "role": message_role_string(message.role),
            "content": openai_responses_content_parts(&message.parts)?,
        })),
        ResponseItem::ToolCall { call } => Ok(json!({
            "type": "function_call",
            "call_id": call.call_id,
            "name": call.name,
            "arguments": call.arguments.to_string(),
        })),
        ResponseItem::ToolResult { result } => Ok(json!({
            "type": "function_call_output",
            "call_id": result.call_id,
            "name": result.name,
            "output": result.output,
            "is_error": result.is_error,
        })),
        ResponseItem::Reasoning { text } => Ok(json!({
            "type": "reasoning",
            "summary": text,
        })),
        ResponseItem::Refusal { text } => Ok(json!({
            "type": "refusal",
            "text": text,
        })),
    }
}

fn parse_openai_responses_content_parts(
    content: &Value,
) -> Result<Vec<MessagePart>, ProtocolError> {
    match content {
        Value::Null => Ok(Vec::new()),
        Value::String(text) => Ok(vec![MessagePart::Text { text: text.clone() }]),
        Value::Array(parts) => parts
            .iter()
            .map(parse_openai_responses_content_part)
            .collect(),
        _ => Err(ProtocolError::InvalidShape("responses content".into())),
    }
}

fn parse_openai_responses_content_part(part: &Value) -> Result<MessagePart, ProtocolError> {
    Ok(
        match part.get("type").and_then(Value::as_str).unwrap_or_default() {
            "input_text" | "output_text" | "text" => MessagePart::Text {
                text: part
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            "input_image" => {
                if let Some(url) = part.get("image_url").and_then(Value::as_str) {
                    MessagePart::ImageUrl {
                        url: url.to_string(),
                        detail: part
                            .get("detail")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    }
                } else {
                    MessagePart::ImageBase64 {
                        data: part
                            .get("image_base64")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        media_type: part
                            .get("media_type")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    }
                }
            }
            "input_file" => MessagePart::File {
                file_id: part
                    .get("file_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                media_type: part
                    .get("media_type")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                data: part.get("data").and_then(Value::as_str).map(str::to_owned),
                filename: part
                    .get("filename")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
            "function_call" => MessagePart::ToolCall {
                call: ToolCallPart {
                    call_id: part
                        .get("call_id")
                        .or_else(|| part.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: part
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    arguments: parse_maybe_json(
                        part.get("arguments")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(Map::new())),
                    ),
                },
            },
            "function_call_output" => MessagePart::ToolResult {
                result: ToolResultPart {
                    call_id: part
                        .get("call_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: part.get("name").and_then(Value::as_str).map(str::to_owned),
                    output: part.get("output").cloned().unwrap_or(Value::Null),
                    is_error: part
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                },
            },
            "refusal" => MessagePart::Refusal {
                text: part
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            "reasoning" => MessagePart::Reasoning {
                text: part
                    .get("summary")
                    .or_else(|| part.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            _ => MessagePart::Json {
                value: part.clone(),
            },
        },
    )
}

fn openai_responses_content_parts(parts: &[MessagePart]) -> Result<Value, ProtocolError> {
    Ok(Value::Array(
        parts
            .iter()
            .cloned()
            .map(openai_responses_content_part)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn openai_responses_content_part(part: MessagePart) -> Result<Value, ProtocolError> {
    Ok(match part {
        MessagePart::Text { text } => json!({ "type": "input_text", "text": text }),
        MessagePart::ImageUrl { url, detail } => json!({
            "type": "input_image",
            "image_url": url,
            "detail": detail,
        }),
        MessagePart::ImageBase64 { data, media_type } => json!({
            "type": "input_image",
            "image_base64": data,
            "media_type": media_type,
        }),
        MessagePart::Audio {
            data, media_type, ..
        } => json!({
            "type": "input_file",
            "data": data,
            "media_type": media_type.unwrap_or_else(|| "audio/wav".into()),
        }),
        MessagePart::File {
            file_id,
            media_type,
            data,
            filename,
        } => json!({
            "type": "input_file",
            "file_id": file_id,
            "media_type": media_type,
            "data": data,
            "filename": filename,
        }),
        MessagePart::Json { value } => json!({ "type": "input_text", "text": value.to_string() }),
        MessagePart::ToolCall { call } => json!({
            "type": "function_call",
            "call_id": call.call_id,
            "name": call.name,
            "arguments": call.arguments.to_string(),
        }),
        MessagePart::ToolResult { result } => json!({
            "type": "function_call_output",
            "call_id": result.call_id,
            "name": result.name,
            "output": result.output,
            "is_error": result.is_error,
        }),
        MessagePart::Reasoning { text } => json!({ "type": "reasoning", "summary": text }),
        MessagePart::Refusal { text } => json!({ "type": "refusal", "text": text }),
    })
}

fn parse_function_tools(value: Option<&Value>) -> Result<Vec<ToolDefinition>, ProtocolError> {
    let Some(Value::Array(tools)) = value else {
        return Ok(Vec::new());
    };

    tools
        .iter()
        .map(|tool| {
            let function = if tool.get("function").is_some() {
                tool.get("function").unwrap_or(tool)
            } else {
                tool
            };
            Ok(ToolDefinition {
                name: required_str(function, "name")?.to_string(),
                description: function
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                input_schema: function
                    .get("parameters")
                    .or_else(|| function.get("input_schema"))
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                strict: function
                    .get("strict")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                vendor_extensions: VendorExtensions::new(),
            })
        })
        .collect()
}

fn emit_function_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                    "strict": tool.strict,
                }
            })
        })
        .collect()
}

fn emit_openai_responses_function_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
                "strict": tool.strict,
            })
        })
        .collect()
}

fn parse_openai_chat_structured_output(body: &Value) -> Option<StructuredOutputConfig> {
    let response_format = body.get("response_format")?;
    if response_format.get("type").and_then(Value::as_str)? != "json_schema" {
        return None;
    }
    let schema = response_format.get("json_schema")?;
    Some(StructuredOutputConfig {
        name: schema
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned),
        schema: schema.get("schema").cloned().unwrap_or(Value::Null),
        strict: schema
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn parse_openai_chat_message(value: &Value) -> Result<Message, ProtocolError> {
    let role = parse_message_role(required_str(value, "role")?);
    if role == MessageRole::Tool {
        return Ok(Message {
            role,
            parts: vec![MessagePart::ToolResult {
                result: ToolResultPart {
                    call_id: value
                        .get("tool_call_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: None,
                    output: content_to_json_string(value.get("content").unwrap_or(&Value::Null)),
                    is_error: false,
                },
            }],
            raw_message: Some(value.to_string()),
            vendor_extensions: VendorExtensions::new(),
        });
    }

    let mut parts = parse_openai_chat_content(value.get("content").unwrap_or(&Value::Null))?;
    if let Some(tool_calls) = value.get("tool_calls").and_then(Value::as_array) {
        for tool_call in tool_calls {
            parts.push(MessagePart::ToolCall {
                call: ToolCallPart {
                    call_id: tool_call
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: tool_call
                        .get("function")
                        .and_then(|value| value.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    arguments: parse_maybe_json(
                        tool_call
                            .get("function")
                            .and_then(|value| value.get("arguments"))
                            .cloned()
                            .unwrap_or_else(|| Value::Object(Map::new())),
                    ),
                },
            });
        }
    }

    Ok(Message {
        role,
        parts,
        raw_message: Some(value.to_string()),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn openai_chat_message_json(message: Message) -> Result<Value, ProtocolError> {
    if message.role == MessageRole::Tool {
        let Some(tool_result) = message.parts.iter().find_map(|part| match part {
            MessagePart::ToolResult { result } => Some(result.clone()),
            _ => None,
        }) else {
            return Err(ProtocolError::UnsupportedFeature(
                "tool message without tool result part".into(),
            ));
        };
        return Ok(json!({
            "role": "tool",
            "tool_call_id": tool_result.call_id,
            "content": value_to_text(tool_result.output),
        }));
    }

    let content_parts = message
        .parts
        .iter()
        .filter(|part| !matches!(part, MessagePart::ToolCall { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let tool_calls = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::ToolCall { call } => Some(json!({
                "id": call.call_id,
                "type": "function",
                "function": {
                    "name": call.name,
                    "arguments": call.arguments.to_string(),
                }
            })),
            _ => None,
        })
        .collect::<Vec<_>>();

    let content = openai_chat_content(&content_parts)?;
    let mut map = Map::new();
    map.insert(
        "role".into(),
        Value::String(message_role_string(message.role).to_string()),
    );
    map.insert("content".into(), content);
    if !tool_calls.is_empty() {
        map.insert("tool_calls".into(), Value::Array(tool_calls));
    }
    Ok(Value::Object(map))
}

fn parse_openai_chat_content(content: &Value) -> Result<Vec<MessagePart>, ProtocolError> {
    match content {
        Value::Null => Ok(Vec::new()),
        Value::String(text) => Ok(vec![MessagePart::Text { text: text.clone() }]),
        Value::Array(parts) => parts
            .iter()
            .map(
                |part| match part.get("type").and_then(Value::as_str).unwrap_or("text") {
                    "text" => Ok(MessagePart::Text {
                        text: part
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    }),
                    "image_url" => Ok(MessagePart::ImageUrl {
                        url: part
                            .get("image_url")
                            .and_then(|value| value.get("url"))
                            .or_else(|| part.get("image_url"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        detail: part
                            .get("image_url")
                            .and_then(|value| value.get("detail"))
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    }),
                    "refusal" => Ok(MessagePart::Refusal {
                        text: part
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    }),
                    _ => Ok(MessagePart::Json {
                        value: part.clone(),
                    }),
                },
            )
            .collect(),
        _ => Err(ProtocolError::InvalidShape("chat message content".into())),
    }
}

fn openai_chat_content(parts: &[MessagePart]) -> Result<Value, ProtocolError> {
    if parts.is_empty() {
        return Ok(Value::Null);
    }
    if parts
        .iter()
        .all(|part| matches!(part, MessagePart::Text { .. }))
    {
        return Ok(Value::String(
            parts
                .iter()
                .filter_map(MessagePart::plain_text)
                .collect::<Vec<_>>()
                .join(""),
        ));
    }
    Ok(Value::Array(
        parts
            .iter()
            .cloned()
            .map(|part| match part {
                MessagePart::Text { text } => Ok(json!({ "type": "text", "text": text })),
                MessagePart::ImageUrl { url, detail } => Ok(json!({
                    "type": "image_url",
                    "image_url": { "url": url, "detail": detail },
                })),
                MessagePart::Json { value } => {
                    Ok(json!({ "type": "text", "text": value.to_string() }))
                }
                MessagePart::Refusal { text } => Ok(json!({ "type": "refusal", "text": text })),
                MessagePart::Reasoning { text } => Ok(json!({ "type": "text", "text": text })),
                MessagePart::ToolResult { result } => Ok(json!({
                    "type": "text",
                    "text": value_to_text(result.output),
                })),
                other => Err(ProtocolError::UnsupportedFeature(format!(
                    "OpenAI Chat content part {:?}",
                    other
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn parse_claude_tools(value: Option<&Value>) -> Result<Vec<ToolDefinition>, ProtocolError> {
    let Some(Value::Array(tools)) = value else {
        return Ok(Vec::new());
    };
    tools
        .iter()
        .map(|tool| {
            Ok(ToolDefinition {
                name: required_str(tool, "name")?.to_string(),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                input_schema: tool
                    .get("input_schema")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                strict: false,
                vendor_extensions: VendorExtensions::new(),
            })
        })
        .collect()
}

fn emit_claude_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.input_schema,
            })
        })
        .collect()
}

fn parse_claude_message(value: &Value) -> Result<Message, ProtocolError> {
    let role = value
        .get("role")
        .and_then(Value::as_str)
        .map(parse_message_role)
        .unwrap_or(MessageRole::Assistant);
    let parts = match value.get("content").unwrap_or(&Value::Null) {
        Value::String(text) => vec![MessagePart::Text { text: text.clone() }],
        Value::Array(blocks) => blocks
            .iter()
            .map(|block| {
                Ok(
                    match block
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                    {
                        "text" => MessagePart::Text {
                            text: block
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        },
                        "tool_use" => MessagePart::ToolCall {
                            call: ToolCallPart {
                                call_id: block
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                name: block
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                arguments: block.get("input").cloned().unwrap_or(Value::Null),
                            },
                        },
                        "tool_result" => MessagePart::ToolResult {
                            result: ToolResultPart {
                                call_id: block
                                    .get("tool_use_id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                name: None,
                                output: content_to_json_string(
                                    block.get("content").unwrap_or(&Value::Null),
                                ),
                                is_error: block
                                    .get("is_error")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                            },
                        },
                        "image" => MessagePart::ImageBase64 {
                            data: block
                                .get("source")
                                .and_then(|value| value.get("data"))
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            media_type: block
                                .get("source")
                                .and_then(|value| value.get("media_type"))
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                        },
                        "document" => MessagePart::File {
                            file_id: None,
                            media_type: block
                                .get("source")
                                .and_then(|value| value.get("media_type"))
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                            data: block
                                .get("source")
                                .and_then(|value| value.get("data"))
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                            filename: None,
                        },
                        _ => MessagePart::Json {
                            value: block.clone(),
                        },
                    },
                )
            })
            .collect::<Result<Vec<_>, ProtocolError>>()?,
        _ => Vec::new(),
    };
    Ok(Message {
        role,
        parts,
        raw_message: Some(value.to_string()),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn claude_message_json(message: Message) -> Result<Value, ProtocolError> {
    let role = match message.role {
        MessageRole::Assistant => "assistant",
        _ => "user",
    };
    Ok(json!({
        "role": role,
        "content": claude_content_parts(&message.parts)?,
    }))
}

fn claude_content_parts(parts: &[MessagePart]) -> Result<Vec<Value>, ProtocolError> {
    parts
        .iter()
        .cloned()
        .map(|part| {
            Ok(match part {
                MessagePart::Text { text }
                | MessagePart::Reasoning { text }
                | MessagePart::Refusal { text } => json!({
                    "type": "text",
                    "text": text,
                }),
                MessagePart::ImageBase64 { data, media_type } => json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type.unwrap_or_else(|| "image/png".into()),
                        "data": data,
                    }
                }),
                MessagePart::ToolCall { call } => json!({
                    "type": "tool_use",
                    "id": call.call_id,
                    "name": call.name,
                    "input": call.arguments,
                }),
                MessagePart::ToolResult { result } => json!({
                    "type": "tool_result",
                    "tool_use_id": result.call_id,
                    "content": value_to_text(result.output),
                    "is_error": result.is_error,
                }),
                MessagePart::Json { value } => json!({
                    "type": "text",
                    "text": value.to_string(),
                }),
                other => {
                    return Err(ProtocolError::UnsupportedFeature(format!(
                        "Claude content part {:?}",
                        other
                    )))
                }
            })
        })
        .collect()
}

fn parse_gemini_instruction(value: &Value) -> Result<String, ProtocolError> {
    let message = parse_gemini_content(value)?;
    Ok(message.plain_text())
}

fn parse_gemini_capabilities(body: &Value) -> Result<CapabilitySet, ProtocolError> {
    let mut capabilities = CapabilitySet::default();
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        for tool in tools {
            if let Some(decls) = tool.get("functionDeclarations").and_then(Value::as_array) {
                for decl in decls {
                    capabilities.tools.push(ToolDefinition {
                        name: required_str(decl, "name")?.to_string(),
                        description: decl
                            .get("description")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        input_schema: decl
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                        strict: false,
                        vendor_extensions: VendorExtensions::new(),
                    });
                }
            }
            if tool.get("codeExecution").is_some() {
                capabilities.builtin_tools.push(BuiltinTool::CodeExecution);
            }
        }
    }
    if let Some(config) = body.get("generationConfig") {
        if let Some(schema) = config.get("responseSchema") {
            capabilities.structured_output = Some(StructuredOutputConfig {
                name: None,
                schema: schema.clone(),
                strict: false,
            });
        }
    }
    Ok(capabilities)
}

fn emit_gemini_tools(capabilities: &CapabilitySet) -> Result<Vec<Value>, ProtocolError> {
    let mut tools = Vec::new();
    if !capabilities.tools.is_empty() {
        tools.push(json!({
            "functionDeclarations": capabilities.tools.iter().map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                })
            }).collect::<Vec<_>>()
        }));
    }
    for builtin in &capabilities.builtin_tools {
        match builtin {
            BuiltinTool::CodeExecution => tools.push(json!({ "codeExecution": {} })),
            other => {
                return Err(ProtocolError::UnsupportedFeature(format!(
                    "Gemini builtin tool {:?}",
                    other
                )))
            }
        }
    }
    Ok(tools)
}

fn parse_gemini_content(value: &Value) -> Result<Message, ProtocolError> {
    let role = match value.get("role").and_then(Value::as_str).unwrap_or("user") {
        "model" => MessageRole::Assistant,
        "user" => MessageRole::User,
        "tool" => MessageRole::Tool,
        "system" => MessageRole::System,
        other => parse_message_role(other),
    };
    let parts = value
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("parts".into()))?
        .iter()
        .map(|part| {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                return Ok(MessagePart::Text {
                    text: text.to_string(),
                });
            }
            if let Some(inline) = part.get("inlineData") {
                let media_type = inline
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let data = inline
                    .get("data")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if media_type.as_deref().unwrap_or("").starts_with("image/") {
                    return Ok(MessagePart::ImageBase64 { data, media_type });
                }
                if media_type.as_deref().unwrap_or("").starts_with("audio/") {
                    return Ok(MessagePart::Audio {
                        data,
                        media_type,
                        transcript: None,
                    });
                }
                return Ok(MessagePart::File {
                    file_id: None,
                    media_type,
                    data: Some(data),
                    filename: None,
                });
            }
            if let Some(file_data) = part.get("fileData") {
                return Ok(MessagePart::File {
                    file_id: file_data
                        .get("fileUri")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    media_type: file_data
                        .get("mimeType")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    data: None,
                    filename: None,
                });
            }
            if let Some(function_call) = part.get("functionCall") {
                return Ok(MessagePart::ToolCall {
                    call: ToolCallPart {
                        call_id: function_call
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        name: function_call
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        arguments: function_call
                            .get("args")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(Map::new())),
                    },
                });
            }
            if let Some(function_response) = part.get("functionResponse") {
                return Ok(MessagePart::ToolResult {
                    result: ToolResultPart {
                        call_id: function_response
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        name: function_response
                            .get("name")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        output: function_response
                            .get("response")
                            .cloned()
                            .unwrap_or(Value::Null),
                        is_error: false,
                    },
                });
            }
            Ok(MessagePart::Json {
                value: part.clone(),
            })
        })
        .collect::<Result<Vec<_>, ProtocolError>>()?;
    Ok(Message {
        role,
        parts,
        raw_message: Some(value.to_string()),
        vendor_extensions: VendorExtensions::new(),
    })
}

fn parse_gemini_candidate(value: &Value) -> Result<Message, ProtocolError> {
    parse_gemini_content(value.get("content").unwrap_or(value))
}

fn gemini_content_json(message: Message) -> Result<Value, ProtocolError> {
    let role = match message.role {
        MessageRole::Assistant => "model",
        MessageRole::System => "system",
        MessageRole::Tool => "user",
        _ => "user",
    };
    Ok(json!({
        "role": role,
        "parts": gemini_parts(&message.parts)?,
    }))
}

fn gemini_parts(parts: &[MessagePart]) -> Result<Vec<Value>, ProtocolError> {
    parts
        .iter()
        .cloned()
        .map(|part| {
            Ok(match part {
                MessagePart::Text { text }
                | MessagePart::Reasoning { text }
                | MessagePart::Refusal { text } => json!({ "text": text }),
                MessagePart::ImageBase64 { data, media_type } => json!({
                    "inlineData": {
                        "mimeType": media_type.unwrap_or_else(|| "image/png".into()),
                        "data": data,
                    }
                }),
                MessagePart::ImageUrl { url, detail: _ } => json!({
                    "fileData": {
                        "fileUri": url,
                        "mimeType": "image/*",
                    }
                }),
                MessagePart::Audio { data, media_type, .. } => json!({
                    "inlineData": {
                        "mimeType": media_type.unwrap_or_else(|| "audio/wav".into()),
                        "data": data,
                    }
                }),
                MessagePart::File {
                    file_id,
                    media_type,
                    data,
                    filename: _,
                } => {
                    if let Some(data) = data {
                        json!({
                            "inlineData": {
                                "mimeType": media_type.unwrap_or_else(|| "application/octet-stream".into()),
                                "data": data,
                            }
                        })
                    } else {
                        json!({
                            "fileData": {
                                "fileUri": file_id.unwrap_or_default(),
                                "mimeType": media_type.unwrap_or_else(|| "application/octet-stream".into()),
                            }
                        })
                    }
                }
                MessagePart::ToolCall { call } => json!({
                    "functionCall": {
                        "id": call.call_id,
                        "name": call.name,
                        "args": call.arguments,
                    }
                }),
                MessagePart::ToolResult { result } => json!({
                    "functionResponse": {
                        "id": result.call_id,
                        "name": result.name.unwrap_or_else(|| "tool".into()),
                        "response": result.output,
                    }
                }),
                MessagePart::Json { value } => json!({ "text": value.to_string() }),
            })
        })
        .collect()
}

fn parse_generation(
    max_output_tokens: Option<u64>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    top_k: Option<u64>,
    stop_sequences: Vec<String>,
    presence_penalty: Option<f64>,
    frequency_penalty: Option<f64>,
    seed: Option<u64>,
) -> GenerationConfig {
    GenerationConfig {
        max_output_tokens: max_output_tokens.map(|value| value as u32),
        temperature: temperature.map(|value| value as f32),
        top_p: top_p.map(|value| value as f32),
        top_k: top_k.map(|value| value as u32),
        stop_sequences,
        presence_penalty: presence_penalty.map(|value| value as f32),
        frequency_penalty: frequency_penalty.map(|value| value as f32),
        seed,
        vendor_extensions: VendorExtensions::new(),
    }
}

fn emit_generation_common(
    map: &mut Map<String, Value>,
    generation: &GenerationConfig,
    responses_style: bool,
) {
    if let Some(max_tokens) = generation.max_output_tokens {
        map.insert(
            if responses_style {
                "max_output_tokens".into()
            } else {
                "max_tokens".into()
            },
            Value::from(max_tokens),
        );
    }
    if let Some(temperature) = generation.temperature {
        map.insert("temperature".into(), Value::from(temperature));
    }
    if let Some(top_p) = generation.top_p {
        map.insert("top_p".into(), Value::from(top_p));
    }
    if let Some(top_k) = generation.top_k {
        map.insert("top_k".into(), Value::from(top_k));
    }
    if !generation.stop_sequences.is_empty() {
        map.insert(
            "stop".into(),
            Value::Array(
                generation
                    .stop_sequences
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if let Some(presence_penalty) = generation.presence_penalty {
        map.insert("presence_penalty".into(), Value::from(presence_penalty));
    }
    if let Some(frequency_penalty) = generation.frequency_penalty {
        map.insert("frequency_penalty".into(), Value::from(frequency_penalty));
    }
    if let Some(seed) = generation.seed {
        map.insert("seed".into(), Value::from(seed));
    }
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, ProtocolError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError::MissingField(field.into()))
}

fn parse_message_role(role: &str) -> MessageRole {
    match role {
        "developer" => MessageRole::Developer,
        "system" => MessageRole::System,
        "assistant" | "model" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        _ => MessageRole::User,
    }
}

fn message_role_string(role: MessageRole) -> &'static str {
    match role {
        MessageRole::Developer => "developer",
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

fn string_or_array(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn object_to_extensions(value: Option<&Value>) -> VendorExtensions {
    value
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn extensions_to_object(value: &VendorExtensions) -> Map<String, Value> {
    value
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn parse_builtin_tool(tool_type: &str, value: &Value) -> BuiltinTool {
    match tool_type {
        "web_search" => BuiltinTool::WebSearch,
        "file_search" => BuiltinTool::FileSearch,
        "code_interpreter" | "code_execution" => BuiltinTool::CodeExecution,
        "computer_use" => BuiltinTool::ComputerUse,
        "url_context" => BuiltinTool::UrlContext,
        "maps" => BuiltinTool::Maps,
        "mcp" => BuiltinTool::Mcp {
            server_label: value
                .get("server_label")
                .and_then(Value::as_str)
                .map(str::to_owned),
        },
        other => BuiltinTool::Vendor {
            name: other.to_string(),
            payload: value.clone(),
        },
    }
}

fn openai_builtin_tool_json(tool: BuiltinTool) -> Result<Value, ProtocolError> {
    Ok(match tool {
        BuiltinTool::WebSearch => json!({ "type": "web_search" }),
        BuiltinTool::FileSearch => json!({ "type": "file_search" }),
        BuiltinTool::CodeExecution => json!({ "type": "code_interpreter" }),
        BuiltinTool::ComputerUse => json!({ "type": "computer_use" }),
        BuiltinTool::UrlContext => json!({ "type": "url_context" }),
        BuiltinTool::Maps => json!({ "type": "maps" }),
        BuiltinTool::Mcp { server_label } => json!({
            "type": "mcp",
            "server_label": server_label,
        }),
        BuiltinTool::Vendor { name, payload } => {
            let mut object = payload.as_object().cloned().unwrap_or_default();
            object.insert("type".into(), Value::String(name));
            Value::Object(object)
        }
    })
}

fn parse_json_schema_format(value: &Value) -> Option<StructuredOutputConfig> {
    if value.get("type").and_then(Value::as_str)? != "json_schema" {
        return None;
    }
    Some(StructuredOutputConfig {
        name: value.get("name").and_then(Value::as_str).map(str::to_owned),
        schema: value.get("schema").cloned().unwrap_or(Value::Null),
        strict: value
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn parse_maybe_json(value: Value) -> Value {
    match value {
        Value::String(string) => serde_json::from_str(&string).unwrap_or(Value::String(string)),
        other => other,
    }
}

fn content_to_json_string(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(text.clone()),
        Value::Array(values) => {
            let text = values
                .iter()
                .filter_map(|value| value.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("");
            if text.is_empty() {
                Value::String(value.to_string())
            } else {
                Value::String(text)
            }
        }
        other => other.clone(),
    }
}

fn value_to_text(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

fn parse_finish_reason(value: &Value) -> Option<FinishReason> {
    let raw = value.as_str()?;
    Some(match raw {
        "stop" | "end_turn" | "STOP" => FinishReason::Stop,
        "length" | "max_tokens" | "MAX_TOKENS" => FinishReason::Length,
        "tool_calls" | "tool_use" | "function_call" => FinishReason::ToolCall,
        "content_filter" => FinishReason::ContentFilter,
        "cancelled" => FinishReason::Cancelled,
        "error" | "ERROR" => FinishReason::Error,
        other => FinishReason::Other(other.to_string()),
    })
}

fn finish_reason_string(reason: &FinishReason) -> String {
    match reason {
        FinishReason::Stop => "stop".into(),
        FinishReason::Length => "length".into(),
        FinishReason::ToolCall => "tool_calls".into(),
        FinishReason::ContentFilter => "content_filter".into(),
        FinishReason::Cancelled => "cancelled".into(),
        FinishReason::Error => "error".into(),
        FinishReason::Other(value) => value.clone(),
    }
}

fn finish_reason_string_upper_camel(reason: &FinishReason) -> String {
    match reason {
        FinishReason::Stop => "STOP".into(),
        FinishReason::Length => "MAX_TOKENS".into(),
        FinishReason::ToolCall => "TOOL_USE".into(),
        FinishReason::ContentFilter => "SAFETY".into(),
        FinishReason::Cancelled => "CANCELLED".into(),
        FinishReason::Error => "ERROR".into(),
        FinishReason::Other(value) => value.clone(),
    }
}

fn response_items_from_message(message: &Message) -> Vec<ResponseItem> {
    let mut items = vec![ResponseItem::Message {
        message: message.clone(),
    }];
    for part in &message.parts {
        match part {
            MessagePart::ToolCall { call } => {
                items.push(ResponseItem::ToolCall { call: call.clone() })
            }
            MessagePart::ToolResult { result } => items.push(ResponseItem::ToolResult {
                result: result.clone(),
            }),
            MessagePart::Reasoning { text } => {
                items.push(ResponseItem::Reasoning { text: text.clone() })
            }
            MessagePart::Refusal { text } => {
                items.push(ResponseItem::Refusal { text: text.clone() })
            }
            _ => {}
        }
    }
    items
}

fn response_item_as_message(item: &ResponseItem) -> Option<&Message> {
    match item {
        ResponseItem::Message { message } => Some(message),
        _ => None,
    }
}

fn collect_message_text(messages: &[Message]) -> String {
    messages
        .iter()
        .map(Message::plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

fn response_output_items(response: &LlmResponse) -> Vec<ResponseItem> {
    if !response.output.is_empty() {
        return response.output.clone();
    }
    response
        .messages
        .iter()
        .flat_map(response_items_from_message)
        .collect()
}

fn assistant_message_from_response(response: &LlmResponse) -> Option<Message> {
    if let Some(message) = response
        .messages
        .iter()
        .find(|message| message.role == MessageRole::Assistant)
    {
        return Some(message.clone());
    }

    let mut parts = Vec::new();
    for item in response_output_items(response) {
        match item {
            ResponseItem::Message { message } => parts.extend(message.parts),
            ResponseItem::ToolCall { call } => parts.push(MessagePart::ToolCall { call }),
            ResponseItem::ToolResult { result } => parts.push(MessagePart::ToolResult { result }),
            ResponseItem::Reasoning { text } => parts.push(MessagePart::Reasoning { text }),
            ResponseItem::Refusal { text } => parts.push(MessagePart::Refusal { text }),
        }
    }
    if parts.is_empty() && response.content_text.is_empty() {
        None
    } else {
        if parts.is_empty() {
            parts.push(MessagePart::Text {
                text: response.content_text.clone(),
            });
        }
        Some(Message {
            role: MessageRole::Assistant,
            parts,
            raw_message: None,
            vendor_extensions: VendorExtensions::new(),
        })
    }
}

fn chat_messages_with_instructions(request: &LlmRequest) -> Vec<Message> {
    let mut messages = request.normalized_messages();
    if let Some(instructions) = &request.instructions {
        messages.retain(|message| {
            !matches!(message.role, MessageRole::System | MessageRole::Developer)
        });
        messages.insert(
            0,
            Message::text(MessageRole::Developer, instructions.clone()),
        );
    }
    messages
}

fn request_messages_for_separate_instruction_protocol(request: &LlmRequest) -> Vec<Message> {
    request
        .normalized_messages()
        .into_iter()
        .filter(|message| !matches!(message.role, MessageRole::System | MessageRole::Developer))
        .collect()
}

fn request_items_for_instructionless_protocol(request: &LlmRequest) -> Vec<RequestItem> {
    request
        .normalized_input()
        .into_iter()
        .filter(|item| match item {
            RequestItem::Message { message } => {
                !matches!(message.role, MessageRole::System | MessageRole::Developer)
            }
            RequestItem::ToolResult { .. } => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let raw = emit_request(ProviderProtocol::OpenAiResponses, &request)
            .expect("emit responses request");
        let body: Value = serde_json::from_str(&raw).expect("parse emitted body");

        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["name"], "lookup_weather");
        assert!(body["tools"][0].get("function").is_none());
        assert_eq!(body["tools"][0]["strict"], true);
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
        let parsed = parse_request(ProviderProtocol::GeminiGenerateContent, &raw)
            .expect("parse gemini request");

        assert_eq!(parsed.model, "gemini-2.5-pro");
        assert_eq!(parsed.generation.max_output_tokens, Some(16));
    }
}
