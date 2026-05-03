use serde_json::{json, Map, Value};

use crate::types::{
    Message, MessagePart, MessageRole, StructuredOutputConfig, ToolCallPart, ToolResultPart,
    VendorExtensions,
};

use super::super::super::common::{
    content_to_json_string, message_role_string, parse_maybe_json, parse_message_role,
    required_str, value_to_text,
};
use super::super::super::ProtocolError;

pub(in crate::protocol::openai) fn parse_openai_chat_structured_output(
    body: &Value,
) -> Option<StructuredOutputConfig> {
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

pub(in crate::protocol::openai) fn parse_openai_chat_message(
    value: &Value,
) -> Result<Message, ProtocolError> {
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

pub(in crate::protocol::openai) fn openai_chat_message_json(
    message: Message,
) -> Result<Value, ProtocolError> {
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

pub(in crate::protocol::openai) fn parse_openai_chat_content(
    content: &Value,
) -> Result<Vec<MessagePart>, ProtocolError> {
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

pub(in crate::protocol::openai) fn openai_chat_content(
    parts: &[MessagePart],
) -> Result<Value, ProtocolError> {
    if parts.is_empty() {
        return Ok(Value::Null);
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
