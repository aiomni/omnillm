use serde_json::{json, Map, Value};

use crate::types::{
    BuiltinTool, FinishReason, GenerationConfig, LlmRequest, LlmResponse, Message, MessagePart,
    MessageRole, PromptCacheKey, PromptCachePolicy, RequestItem, ResponseItem,
    StructuredOutputConfig, VendorExtensions,
};

use super::ProtocolError;

pub(super) fn parse_generation(
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

pub(super) fn emit_generation_common(
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
pub(super) fn value_contains_cache_control(value: &Value) -> bool {
    match value {
        Value::Object(object) => object
            .iter()
            .any(|(key, value)| key == "cache_control" || value_contains_cache_control(value)),
        Value::Array(values) => values.iter().any(value_contains_cache_control),
        _ => false,
    }
}

pub(super) fn find_cache_control_ttl(value: &Value) -> Option<&str> {
    match value {
        Value::Object(object) => {
            if let Some(ttl) = object
                .get("cache_control")
                .and_then(|cache_control| cache_control.get("ttl"))
                .and_then(Value::as_str)
            {
                return Some(ttl);
            }
            object.values().find_map(find_cache_control_ttl)
        }
        Value::Array(values) => values.iter().find_map(find_cache_control_ttl),
        _ => None,
    }
}

pub(super) fn prompt_cache_key_value(key: &PromptCacheKey, request: &LlmRequest) -> String {
    match key {
        PromptCacheKey::Explicit { value } => value.clone(),
        PromptCacheKey::StablePrefixHash {
            namespace,
            tenant_scope,
        } => {
            let fingerprint = json!({
                "model": &request.model,
                "instructions": request.normalized_instructions(),
                "tools": &request.capabilities.tools,
                "input": request.normalized_input(),
            });
            let hash = fnv1a64(fingerprint.to_string().as_bytes());
            match tenant_scope {
                Some(scope) => format!("{}:{}:{hash:016x}", namespace, scope),
                None => format!("{}:{hash:016x}", namespace),
            }
        }
    }
}

pub(super) fn prompt_cache_vendor_extensions(policy: &PromptCachePolicy) -> &VendorExtensions {
    match policy {
        PromptCachePolicy::Disabled => empty_vendor_extensions(),
        PromptCachePolicy::BestEffort {
            vendor_extensions, ..
        }
        | PromptCachePolicy::Required {
            vendor_extensions, ..
        } => vendor_extensions,
    }
}

pub(super) fn empty_vendor_extensions() -> &'static VendorExtensions {
    static EMPTY: std::sync::OnceLock<VendorExtensions> = std::sync::OnceLock::new();
    EMPTY.get_or_init(VendorExtensions::new)
}
pub(super) fn nested_u32(value: &Value, path: &[&str]) -> Option<u32> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    value_as_u32(current)
}

pub(super) fn value_as_u32(value: &Value) -> Option<u32> {
    value.as_u64().map(|value| value as u32)
}

pub(super) fn fnv1a64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(super) fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, ProtocolError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError::MissingField(field.into()))
}

pub(super) fn parse_message_role(role: &str) -> MessageRole {
    match role {
        "developer" => MessageRole::Developer,
        "system" => MessageRole::System,
        "assistant" | "model" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        _ => MessageRole::User,
    }
}

pub(super) fn message_role_string(role: MessageRole) -> &'static str {
    match role {
        MessageRole::Developer => "developer",
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

pub(super) fn string_or_array(value: Option<&Value>) -> Vec<String> {
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

pub(super) fn object_to_extensions(value: Option<&Value>) -> VendorExtensions {
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

pub(super) fn extensions_to_object(value: &VendorExtensions) -> Map<String, Value> {
    value
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub(super) fn collect_vendor_extensions(value: &Value, known_fields: &[&str]) -> VendorExtensions {
    let Some(object) = value.as_object() else {
        return VendorExtensions::new();
    };

    object
        .iter()
        .filter(|(key, _)| !known_fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub(super) fn extend_with_vendor_extensions(
    map: &mut Map<String, Value>,
    vendor_extensions: &VendorExtensions,
) {
    for (key, value) in vendor_extensions {
        map.entry(key.clone()).or_insert_with(|| value.clone());
    }
}

pub(super) fn parse_builtin_tool(tool_type: &str, value: &Value) -> BuiltinTool {
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

pub(super) fn openai_builtin_tool_json(tool: BuiltinTool) -> Result<Value, ProtocolError> {
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

pub(super) fn parse_json_schema_format(value: &Value) -> Option<StructuredOutputConfig> {
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

pub(super) fn parse_maybe_json(value: Value) -> Value {
    match value {
        Value::String(string) => serde_json::from_str(&string).unwrap_or(Value::String(string)),
        other => other,
    }
}

pub(super) fn content_to_json_string(value: &Value) -> Value {
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

pub(super) fn value_to_text(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

pub(super) fn parse_finish_reason(value: &Value) -> Option<FinishReason> {
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

pub(super) fn finish_reason_string(reason: &FinishReason) -> String {
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

pub(super) fn finish_reason_string_upper_camel(reason: &FinishReason) -> String {
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

pub(super) fn response_items_from_message(message: &Message) -> Vec<ResponseItem> {
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

pub(super) fn response_item_as_message(item: &ResponseItem) -> Option<&Message> {
    match item {
        ResponseItem::Message { message } => Some(message),
        _ => None,
    }
}

pub(super) fn collect_message_text(messages: &[Message]) -> String {
    messages
        .iter()
        .map(Message::plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn response_output_items(response: &LlmResponse) -> Vec<ResponseItem> {
    if !response.output.is_empty() {
        return response.output.clone();
    }
    response
        .messages
        .iter()
        .flat_map(response_items_from_message)
        .collect()
}

pub(super) fn assistant_message_from_response(response: &LlmResponse) -> Option<Message> {
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

pub(super) fn chat_messages_with_instructions(request: &LlmRequest) -> Vec<Message> {
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

pub(super) fn request_messages_for_separate_instruction_protocol(
    request: &LlmRequest,
) -> Vec<Message> {
    request
        .normalized_messages()
        .into_iter()
        .filter(|message| !matches!(message.role, MessageRole::System | MessageRole::Developer))
        .collect()
}

pub(super) fn request_items_for_instructionless_protocol(request: &LlmRequest) -> Vec<RequestItem> {
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
