use serde_json::{json, Map, Value};

use crate::types::{
    CapabilitySet, Message, MessagePart, MessageRole, ReasoningCapability, RequestItem,
    ResponseItem, ToolCallPart, ToolDefinition, ToolResultPart, VendorExtensions,
};

use super::super::super::common::{
    message_role_string, openai_builtin_tool_json, parse_builtin_tool, parse_json_schema_format,
    parse_maybe_json, parse_message_role, required_str,
};
use super::super::super::ProtocolError;

use super::cache::parse_openai_prompt_cache_policy;
use super::tools::emit_openai_responses_function_tools;

pub(in crate::protocol::openai) fn parse_openai_responses_capabilities(
    body: &Value,
) -> Result<CapabilitySet, ProtocolError> {
    let mut capabilities = CapabilitySet::default();
    capabilities.prompt_cache = parse_openai_prompt_cache_policy(body);
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

pub(in crate::protocol::openai) fn emit_openai_responses_capabilities(
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

pub(in crate::protocol::openai) fn parse_openai_responses_input(
    input: &Value,
) -> Result<Vec<RequestItem>, ProtocolError> {
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

pub(in crate::protocol::openai) fn parse_openai_responses_input_item(
    item: &Value,
) -> Result<RequestItem, ProtocolError> {
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

pub(in crate::protocol::openai) fn openai_responses_input_item(
    item: RequestItem,
) -> Result<Value, ProtocolError> {
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

pub(in crate::protocol::openai) fn parse_openai_responses_output(
    items: &[Value],
) -> Result<Vec<ResponseItem>, ProtocolError> {
    items
        .iter()
        .map(parse_openai_responses_single_output_item)
        .collect()
}

pub(in crate::protocol::openai) fn parse_openai_responses_single_output_item(
    item: &Value,
) -> Result<ResponseItem, ProtocolError> {
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

pub(in crate::protocol::openai) fn openai_responses_output_item(
    item: ResponseItem,
) -> Result<Value, ProtocolError> {
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

pub(in crate::protocol::openai) fn parse_openai_responses_content_parts(
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

pub(in crate::protocol::openai) fn parse_openai_responses_content_part(
    part: &Value,
) -> Result<MessagePart, ProtocolError> {
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

pub(in crate::protocol::openai) fn openai_responses_content_parts(
    parts: &[MessagePart],
) -> Result<Value, ProtocolError> {
    Ok(Value::Array(
        parts
            .iter()
            .cloned()
            .map(openai_responses_content_part)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

pub(in crate::protocol::openai) fn openai_responses_content_part(
    part: MessagePart,
) -> Result<Value, ProtocolError> {
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
