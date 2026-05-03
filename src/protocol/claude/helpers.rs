use serde_json::{json, Map, Value};

use crate::types::{
    CacheBreakpoint, Message, MessagePart, MessageRole, PromptCachePolicy, PromptCacheRetention,
    PromptCacheUsage, TokenUsage, ToolCallPart, ToolDefinition, ToolResultPart, VendorExtensions,
};

use super::super::common::{
    content_to_json_string, extend_with_vendor_extensions, find_cache_control_ttl, nested_u32,
    parse_message_role, prompt_cache_vendor_extensions, required_str, value_as_u32,
    value_contains_cache_control, value_to_text,
};
use super::super::ProtocolError;

pub(in crate::protocol::claude) fn parse_claude_tools(
    value: Option<&Value>,
) -> Result<Vec<ToolDefinition>, ProtocolError> {
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

pub(in crate::protocol::claude) fn emit_claude_tools(tools: &[ToolDefinition]) -> Vec<Value> {
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

pub(in crate::protocol::claude) fn apply_claude_prompt_cache_policy(
    tools: &mut [Value],
    system: Option<&mut Value>,
    messages: &mut [Value],
    policy: Option<&PromptCachePolicy>,
) -> Result<(), ProtocolError> {
    let Some(policy) = policy.filter(|policy| !policy.is_disabled()) else {
        return Ok(());
    };

    if policy.key().is_some() && policy.is_required() {
        return Err(ProtocolError::UnsupportedFeature(
            "Claude prompt cache does not support explicit cache keys".into(),
        ));
    }

    let cache_control = claude_cache_control_json(policy);
    let applied = match policy.breakpoint() {
        CacheBreakpoint::Auto => {
            if !tools.is_empty() {
                apply_cache_control_to_value(
                    tools.last_mut().expect("checked non-empty"),
                    cache_control,
                )
            } else if let Some(system) = system {
                apply_cache_control_to_system(system, cache_control)
            } else {
                false
            }
        }
        CacheBreakpoint::EndOfTools => tools
            .last_mut()
            .map(|tool| apply_cache_control_to_value(tool, cache_control))
            .unwrap_or(false),
        CacheBreakpoint::EndOfInstructions => system
            .map(|system| apply_cache_control_to_system(system, cache_control))
            .unwrap_or(false),
        CacheBreakpoint::EndOfMessage { index } => messages
            .get_mut(index)
            .map(|message| apply_cache_control_to_message(message, None, cache_control))
            .unwrap_or(false),
        CacheBreakpoint::EndOfContentBlock {
            message_index,
            part_index,
        } => messages
            .get_mut(message_index)
            .map(|message| apply_cache_control_to_message(message, Some(part_index), cache_control))
            .unwrap_or(false),
    };

    if !applied && policy.is_required() {
        return Err(ProtocolError::UnsupportedFeature(
            "Claude prompt cache breakpoint cannot be represented for this request".into(),
        ));
    }

    Ok(())
}

pub(in crate::protocol::claude) fn claude_cache_control_json(policy: &PromptCachePolicy) -> Value {
    let mut map = Map::new();
    map.insert("type".into(), Value::String("ephemeral".into()));
    match policy.retention() {
        PromptCacheRetention::ProviderDefault => {}
        PromptCacheRetention::Short => {
            map.insert("ttl".into(), Value::String("5m".into()));
        }
        PromptCacheRetention::Long => {
            map.insert("ttl".into(), Value::String("1h".into()));
        }
    }
    extend_with_vendor_extensions(&mut map, prompt_cache_vendor_extensions(policy));
    Value::Object(map)
}

pub(in crate::protocol::claude) fn apply_cache_control_to_value(
    value: &mut Value,
    cache_control: Value,
) -> bool {
    let Some(object) = value.as_object_mut() else {
        return false;
    };
    object.insert("cache_control".into(), cache_control);
    true
}

pub(in crate::protocol::claude) fn apply_cache_control_to_system(
    system: &mut Value,
    cache_control: Value,
) -> bool {
    match system {
        Value::String(text) => {
            let text = text.clone();
            *system = json!([{ "type": "text", "text": text, "cache_control": cache_control }]);
            true
        }
        Value::Array(blocks) => blocks
            .last_mut()
            .map(|block| apply_cache_control_to_value(block, cache_control))
            .unwrap_or(false),
        _ => false,
    }
}

pub(in crate::protocol::claude) fn apply_cache_control_to_message(
    message: &mut Value,
    part_index: Option<usize>,
    cache_control: Value,
) -> bool {
    let Some(parts) = message.get_mut("content").and_then(Value::as_array_mut) else {
        return false;
    };
    let target_index = part_index.unwrap_or_else(|| parts.len().saturating_sub(1));
    if parts.is_empty() {
        return false;
    }
    parts
        .get_mut(target_index)
        .map(|part| apply_cache_control_to_value(part, cache_control))
        .unwrap_or(false)
}

pub(in crate::protocol::claude) fn parse_claude_message(
    value: &Value,
) -> Result<Message, ProtocolError> {
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

pub(in crate::protocol::claude) fn claude_message_json(
    message: Message,
) -> Result<Value, ProtocolError> {
    let role = match message.role {
        MessageRole::Assistant => "assistant",
        _ => "user",
    };
    Ok(json!({
        "role": role,
        "content": claude_content_parts(&message.parts)?,
    }))
}

pub(in crate::protocol::claude) fn claude_content_parts(
    parts: &[MessagePart],
) -> Result<Vec<Value>, ProtocolError> {
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

pub(in crate::protocol::claude) fn parse_claude_prompt_cache_policy(
    body: &Value,
) -> Option<PromptCachePolicy> {
    if value_contains_cache_control(body) {
        Some(PromptCachePolicy::BestEffort {
            key: None,
            retention: parse_claude_prompt_cache_retention(body),
            breakpoint: CacheBreakpoint::Auto,
            vendor_extensions: VendorExtensions::new(),
        })
    } else {
        None
    }
}

pub(in crate::protocol::claude) fn parse_claude_prompt_cache_retention(
    value: &Value,
) -> PromptCacheRetention {
    find_cache_control_ttl(value)
        .map(|ttl| match ttl {
            "1h" => PromptCacheRetention::Long,
            "5m" => PromptCacheRetention::Short,
            _ => PromptCacheRetention::ProviderDefault,
        })
        .unwrap_or_default()
}

pub(in crate::protocol::claude) fn parse_claude_prompt_cache_usage(
    usage: Option<&Value>,
) -> Option<PromptCacheUsage> {
    let usage = usage?;
    let cache_creation_short_input_tokens =
        nested_u32(usage, &["cache_creation", "ephemeral_5m_input_tokens"])
            .or_else(|| {
                usage
                    .get("cache_creation_5m_input_tokens")
                    .and_then(value_as_u32)
            })
            .or_else(|| {
                usage
                    .get("cache_creation_short_input_tokens")
                    .and_then(value_as_u32)
            });
    let cache_creation_long_input_tokens =
        nested_u32(usage, &["cache_creation", "ephemeral_1h_input_tokens"])
            .or_else(|| {
                usage
                    .get("cache_creation_1h_input_tokens")
                    .and_then(value_as_u32)
            })
            .or_else(|| {
                usage
                    .get("cache_creation_long_input_tokens")
                    .and_then(value_as_u32)
            });
    let cache_creation_input_tokens = usage
        .get("cache_creation_input_tokens")
        .and_then(value_as_u32);

    let prompt_cache = PromptCacheUsage {
        cached_input_tokens: None,
        cache_read_input_tokens: usage.get("cache_read_input_tokens").and_then(value_as_u32),
        cache_creation_input_tokens,
        cache_creation_short_input_tokens,
        cache_creation_long_input_tokens,
        vendor_extensions: VendorExtensions::new(),
    };
    (!prompt_cache.is_empty()).then_some(prompt_cache)
}

pub(in crate::protocol::claude) fn claude_usage_json(usage: &TokenUsage) -> Value {
    let mut map = Map::new();
    map.insert("input_tokens".into(), Value::from(usage.prompt_tokens));
    map.insert("output_tokens".into(), Value::from(usage.completion_tokens));
    if let Some(prompt_cache) = &usage.prompt_cache {
        if let Some(value) = prompt_cache.cache_read_input_tokens {
            map.insert("cache_read_input_tokens".into(), Value::from(value));
        }
        if let Some(value) = prompt_cache.cache_creation_input_tokens {
            map.insert("cache_creation_input_tokens".into(), Value::from(value));
        }
        if let Some(value) = prompt_cache.cache_creation_short_input_tokens {
            map.insert("cache_creation_5m_input_tokens".into(), Value::from(value));
        }
        if let Some(value) = prompt_cache.cache_creation_long_input_tokens {
            map.insert("cache_creation_1h_input_tokens".into(), Value::from(value));
        }
    }
    Value::Object(map)
}
